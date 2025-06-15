mod cli;
mod local_k8s;

use local_k8s::pods::pods_exec_log;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cmd = cli::command::PodCli::new();
    pods_exec_log(cmd.command).await?;
    Ok(())
}
