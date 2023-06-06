use futures::StreamExt;
use k8s_openapi::api::core::v1::Pod;

use kube::{
    api::{Api, AttachParams, AttachedProcess, ListParams},
    Client,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client: Client = Client::try_default().await?;

    let pods_all_ns: Api<Pod> = Api::all(client.clone());

    //
    let list_params: ListParams = ListParams::default().labels("");
    //
    let pod_list = pods_all_ns.list(&list_params).await?;

    let cmd = vec!["cat", "/etc/os-release"];
    let container_name = "app";

    for pod in pod_list {
        if get_pod_with_container(&pod, container_name) {
            let name = pod.metadata.name.as_ref().unwrap();
            let ns = pod.metadata.namespace.as_ref().unwrap();
            let kube_pod: Api<Pod> = Api::namespaced(client.clone(), ns.as_str());

            let attached = kube_pod
                .exec(&name, cmd.clone(), &AttachParams::default().stderr(false))
                .await?;

            let output = get_output(attached).await;

            let lines = output.lines();
            for line in lines {
                if line.starts_with("ID=") {
                    println!(
                        "ns={} pod={} get os={}",
                        ns,
                        name,
                        line.strip_prefix("ID=").unwrap()
                    );
                    break;
                }
            }
        }
    }

    Ok(())
}

async fn get_output(mut attached: AttachedProcess) -> String {
    let stdout = tokio_util::io::ReaderStream::new(attached.stdout().unwrap());
    let out: String = stdout
        .filter_map(|r| async { r.ok().and_then(|v| String::from_utf8(v.to_vec()).ok()) })
        .collect::<Vec<_>>()
        .await
        .join("");
    attached.join().await.unwrap();
    out
}

fn get_pod_with_container(p: &Pod, name: &str) -> bool {
    for c in p.spec.clone().unwrap().containers {
        if c.name == name {
            return true;
        }
    }
    return false;
}
