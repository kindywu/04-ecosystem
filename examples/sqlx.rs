#[tokio::main]
async fn main() {
    // #[derive(Debug)]
    // struct Account {
    //     id: i32,
    //     name: String,
    // }

    // // let mut conn = <impl sqlx::Executor>;
    // let account = sqlx::query_as!(
    //     Account,
    //     "select * from (select (1) as id, 'Herp Derpinson' as name) accounts where id = ?",
    //     1i32
    // )
    // .fetch_one(&mut conn)
    // .await?;

    // println!("{account:?}");
    // println!("{}: {}", account.id, account.name);
}
