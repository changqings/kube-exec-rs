mod local_k8s;

use std::sync::Arc;

use local_k8s::pods::pods_exec_log;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cmd = Arc::new(vec!["cat", "/etc/os-release"]);
    pods_exec_log(cmd).await?;
    Ok(())
}
