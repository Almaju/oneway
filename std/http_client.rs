#[allow(dead_code)]
async fn oneway_http_get_text(url: String) -> Result<String, reqwest::Error> {
    let resp = reqwest::get(&url).await?;
    resp.text().await
}
