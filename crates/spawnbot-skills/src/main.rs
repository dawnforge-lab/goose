use rmcp::ServiceExt;
use spawnbot_common::paths;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let skills_dir = paths::skills_dir();
    let extensions_dir = paths::extensions_dir();

    let server = spawnbot_skills::server::SkillsServer::new(skills_dir, extensions_dir);
    let transport = rmcp::transport::io::stdio();
    let service = server.serve(transport).await?;
    service.waiting().await?;
    Ok(())
}
