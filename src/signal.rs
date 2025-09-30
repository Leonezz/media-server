pub async fn stop() {
    let _ = tokio::signal::ctrl_c().await;
}
