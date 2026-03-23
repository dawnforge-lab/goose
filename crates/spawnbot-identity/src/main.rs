use rmcp::ServiceExt;
use spawnbot_common::paths;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let workspace_root = std::env::var("SPAWNBOT_WORKSPACE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| paths::spawnbot_home().join("workspace"));

    let server = spawnbot_identity::server::IdentityServer::new(workspace_root);
    let transport = rmcp::transport::io::stdio();
    let service = server.serve(transport).await?;
    service.waiting().await?;
    Ok(())
}
