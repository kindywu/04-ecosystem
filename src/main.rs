// $env:DATABASE_URL = "postgres://kindy:kindy@localhost:5432/shortener"

use anyhow::Result;
use sqlx::{query_as, Executor, FromRow, PgPool, Postgres};

const PG_URL: &str = "postgres://kindy:kindy@localhost:5432/shortener";

#[derive(Debug, FromRow)]
pub struct Url {
    #[sqlx(default)]
    id: String,
    #[sqlx(default)]
    url: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let pool = PgPool::connect(PG_URL).await?;
    // let id = "X_2C4H";
    let id = "id_not_exist";
    if let Some(url) = get_url(&pool, id).await? {
        println!("{}=>{}", url.id, url.url)
    } else {
        println!("{} not found", id)
    }

    let uri = "www.bing.com";

    let id = shorten(&pool, uri).await?;
    println!("id is {}", id);

    Ok(())
}

async fn get_url<'e, E>(executor: E, id: &str) -> Result<Option<Url>>
where
    E: Executor<'e, Database = Postgres>,
{
    let url_record = query_as!(Url, r#"SELECT id, url FROM urls WHERE id = $1"#, id)
        .fetch_optional(executor)
        .await?;
    Ok(url_record)
}

#[derive(Debug, FromRow)]
pub struct Id {
    id: String,
}

async fn shorten<'e, E>(executor: E, uri: &str) -> Result<String>
where
    E: Executor<'e, Database = Postgres>,
{
    let id = nanoid::nanoid!(6);

    let result = query_as!(
        Id,
        r#"
            INSERT INTO urls (id, url) VALUES ($1, $2)
            ON CONFLICT(url) DO UPDATE SET url=EXCLUDED.url RETURNING id
            "#,
        &id,
        &uri
    )
    .fetch_one(executor)
    .await?;
    Ok(result.id)
}
