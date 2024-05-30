use std::sync::Arc;
use thiserror::Error;

use anyhow::Result;
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
    Json, Router,
};
use hyper::StatusCode;
use serde::Deserialize;
use serde_json::json;
use sqlx::{FromRow, PgPool};
use tokio::net::TcpListener;
use tracing::{info, level_filters::LevelFilter, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};
use url::Url;

const HOST: &str = "127.0.0.1:3000";
const PG_URL: &str = "postgres://kindy:kindy@localhost:5432/shortener";
const MAX_SHORTEN_TRY: u8 = 3;
const MAX_ID_LEN: usize = 6; // 6位大小写字母+数字已经形成足够大的取值空间
const UNIQUE_CONSTRAINT_ERROR: &str = "23505"; // PostgreSQL 23505: duplicate key value violates unique constraint

#[tokio::main]
async fn main() -> Result<()> {
    let layer = tracing_subscriber::fmt::Layer::new().with_filter(LevelFilter::INFO);
    tracing_subscriber::registry().with(layer).init();

    // info!("HOST: {}", HOST);
    // info!("PG_URL: {}", PG_URL);

    let shared_state = Arc::new(AppState::try_new(PG_URL).await?);

    let app = Router::new()
        .route("/:id", get(redirect))
        .route("/shortener", post(shorten))
        .with_state(shared_state);

    let listener = TcpListener::bind(HOST).await?;
    info!("URL shortener serve in {HOST}");
    axum::serve(listener, app).await?;
    info!("URL shortener exit");
    Ok(())
}

#[derive(Error, Debug)]
pub enum ShortenError {
    #[error("the length of the id string must be 6")]
    IdIllegal,

    #[error("the id {0} can't not found")]
    IdNotFound(String),

    #[error("url parse error")]
    UrlIllegal(#[from] url::ParseError),

    #[error("the maximum number of attempts is reached")]
    UrlMaxTrySave,

    #[error("database error")]
    DatabaseError(#[from] sqlx::Error),

    #[error("unknown error")]
    Unknown,
}

impl IntoResponse for ShortenError {
    fn into_response(self) -> Response {
        match self {
            ShortenError::IdIllegal | ShortenError::UrlMaxTrySave | ShortenError::UrlIllegal(_) => {
                (StatusCode::UNPROCESSABLE_ENTITY, self.to_string())
            }
            ShortenError::IdNotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            ShortenError::Unknown | ShortenError::DatabaseError(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "server error, try again later".to_string(),
            ),
        }
        .into_response()
    }
}

async fn redirect(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ShortenError> {
    let uri = state.get_url(&id).await?;
    Ok(Redirect::to(format!("https://{}", uri).as_str()))
}

#[derive(Deserialize)]
struct ShortenReq {
    uri: String,
}

async fn shorten(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ShortenReq>,
) -> Result<impl IntoResponse, ShortenError> {
    let id = state.shorten(&req.uri).await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "url": format!("http://{HOST}/{id}")
        })),
    ))
}

#[derive(Debug, FromRow)]
struct UrlRecord {
    #[sqlx(default)]
    id: String,
    #[sqlx(default)]
    url: String,
}
struct AppState {
    pool: PgPool,
}

impl AppState {
    async fn try_new(pg_url: &str) -> Result<Self, ShortenError> {
        let pool = PgPool::connect(pg_url).await?;
        // Create table if not exists
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS urls (
                id CHAR(6) PRIMARY KEY,
                url TEXT NOT NULL UNIQUE
            )
            "#,
        )
        .execute(&pool)
        .await?;
        Ok(Self { pool })
    }

    async fn shorten(&self, url: &str) -> Result<String, ShortenError> {
        let _ = Url::parse(format!("http://{}", url).as_str())?;

        let id = nanoid::nanoid!(MAX_ID_LEN);

        for i in 0..MAX_SHORTEN_TRY {
            let result:Result<UrlRecord,sqlx::Error> = sqlx::query_as("INSERT INTO urls (id, url) VALUES ($1, $2) ON CONFLICT(url) DO UPDATE SET url=EXCLUDED.url RETURNING id")
            .bind(&id).bind(url).fetch_one(&self.pool).await;

            match result {
                Ok(url) => return Ok(url.id),
                Err(sqlx::Error::Database(e)) => {
                    if let Some(code) = e.code() {
                        if code == UNIQUE_CONSTRAINT_ERROR {
                            warn!(
                                "The {} encounter {UNIQUE_CONSTRAINT_ERROR}: unique_violation, try again!",
                                i + 1
                            );
                            continue;
                        }
                    }
                    return Err(ShortenError::Unknown);
                }
                Err(e) => return Err(ShortenError::DatabaseError(e)),
            }
        }

        Err(ShortenError::UrlMaxTrySave)
    }

    async fn get_url(&self, id: &str) -> Result<String, ShortenError> {
        if id.len() != MAX_ID_LEN {
            return Err(ShortenError::IdIllegal);
        }
        if let Some(UrlRecord { id: _, url }) =
            sqlx::query_as("SELECT id, url FROM urls WHERE id=$1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?
        {
            Ok(url)
        } else {
            Err(ShortenError::IdNotFound(id.to_string()))
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use anyhow::Result;
//     use sqlx::PgPool;

//     #[tokio::test]
//     async fn test_sqlx_connect() -> Result<()> {
//         let pool = PgPool::connect(PG_URL).await?;

//         // 当url存在，返回id
//         let id = "000000"; // V4cL8h | www.baidu.com
//         let url = "www.baidu.com";
//         let UrlRecord { id, url: _ } = sqlx::query_as("INSERT INTO urls (id, url) VALUES ($1, $2) ON CONFLICT(url) DO UPDATE SET url=EXCLUDED.url RETURNING id")
//         .bind(id).bind(url).fetch_one(&pool).await?;
//         assert_eq!(id, "V4cL8h");

//         // 当(id,url)存在，返回id
//         let id = "V4cL8h"; // V4cL8h | www.baidu.com
//         let url = "www.baidu.com";
//         let UrlRecord { id, url: _ }  = sqlx::query_as("INSERT INTO urls (id, url) VALUES ($1, $2) ON CONFLICT(url) DO UPDATE SET url=EXCLUDED.url RETURNING id")
//          .bind(id).bind(url).fetch_one(&pool).await?;
//         assert_eq!(id, "V4cL8h");

//         // 当id存在，抛出冲突错误
//         let id = "V4cL8h"; // V4cL8h | www.baidu.com
//         let url = "www.soso.com";
//         let result:Result<UrlRecord,sqlx::Error> = sqlx::query_as("INSERT INTO urls (id, url) VALUES ($1, $2) ON CONFLICT(url) DO UPDATE SET url=EXCLUDED.url RETURNING id")
//         .bind(id).bind(url).fetch_one(&pool).await;

//         match result {
//             Ok(_) => assert!(false, "must be error"),
//             Err(sqlx::Error::Database(e)) => match e.code() {
//                 Some(code) => assert_eq!(code, UNIQUE_CONSTRAINT_ERROR),
//                 None => assert!(false, "must be database error {}", UNIQUE_CONSTRAINT_ERROR),
//             },
//             Err(_) => assert!(false, "must be database error"),
//         }

//         Ok(())
//     }
// }
