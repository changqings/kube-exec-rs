mod local_k8s;

use local_k8s::pods::print_pods_log;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    print_pods_log().await?;
    Ok(())
}
