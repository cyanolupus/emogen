use worker::*;

pub async fn get<D>(ctx: RouteContext<D>, key: &str) -> Option<Vec<u8>> {
    let bucket = ctx.bucket("BUCKET").unwrap();
    let item = bucket.get(key).execute().await.ok()??;
    item.body()?.bytes().await.ok()
}
