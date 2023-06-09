use std::{collections::BTreeMap, thread, time::Duration};

use futures::StreamExt;
use k8s_openapi::api::core::v1::Pod;

use kube::{
    api::{Api, AttachParams, AttachedProcess, ListParams},
    core::ObjectList,
    Client,
};

#[derive(Debug, Clone)]
struct DeploymentPod {
    deploy: String,
    ns: String,
    pod: String,
}
#[derive(Debug, PartialEq, Eq, Clone)]
struct DeploymentNs {
    deploy: String,
    ns: String,
}

struct OsVersion {
    id: String,
    version: String,
}

impl PartialOrd for DeploymentNs {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for DeploymentNs {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.ns
            .cmp(&other.ns)
            .then_with(|| self.deploy.cmp(&other.deploy))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client: Client = Client::try_default().await?;
    let pods_all_ns: Api<Pod> = Api::all(client.clone());

    //
    let list_params: ListParams = ListParams::default().labels("");
    let pod_list = pods_all_ns.list(&list_params).await?;
    let cmd = vec!["cat", "/etc/os-release"];

    //
    let dp_list = get_deploy_one_pod(pod_list);

    for dp in dp_list {
        let name = dp.pod;
        let ns = dp.ns;
        let kube_pod: Api<Pod> = Api::namespaced(client.clone(), ns.as_str());

        let attached = kube_pod
            .exec(
                &name,
                cmd.clone(),
                &AttachParams::default().stderr(true).container("app"),
            )
            .await?;

        let output = get_output(attached).await;

        let lines = output.lines();
        let mut os = OsVersion {
            id: String::new(),
            version: String::new(),
        };
        for line in lines {
            if line.starts_with("ID=") {
                os.id = line.strip_prefix("ID=").unwrap().to_string();
            }
            if line.starts_with("VERSION_ID=") {
                os.version = line.strip_prefix("VERSION_ID=").unwrap().to_string();
            }
        }
        println!(
            "ns={} pod={} get os={} version={}",
            ns, name, os.id, os.version
        );
        thread::sleep(Duration::from_millis(200));
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

fn get_running_pod_with_container(p: &Pod, name: &str) -> bool {
    for c in p.spec.clone().unwrap().containers {
        if c.name == name {
            if let Some(pc) = p.status.clone().unwrap().conditions {
                for st in pc {
                    if st.type_ == "Ready" {
                        return true;
                    }
                }
            }
        }
    }
    false
}

// generate by gpt4
fn get_deploy_one_pod(pod_list: ObjectList<Pod>) -> Vec<DeploymentPod> {
    let mut dp_vec: Vec<DeploymentPod> = Vec::new();
    let container_name = "app";

    for pod in pod_list {
        if let Some(owner_references) = pod.metadata.owner_references.as_ref() {
            if let Some(owner_reference) = owner_references.get(0) {
                if owner_reference.kind == "ReplicaSet" {
                    if let (Some(hash), Some(generate_name)) = (
                        pod.metadata
                            .labels
                            .as_ref()
                            .and_then(|labels| labels.get("pod-template-hash")),
                        pod.metadata.generate_name.as_ref(),
                    ) {
                        let dp_name = generate_name
                            .split(&format!("-{}-", hash))
                            .nth(0)
                            .unwrap()
                            .to_string();

                        if get_running_pod_with_container(&pod, container_name) {
                            let dp = DeploymentPod {
                                pod: pod.metadata.name.clone().unwrap(),
                                deploy: dp_name,
                                ns: pod.metadata.namespace.clone().unwrap(),
                            };
                            dp_vec.push(dp)
                        }
                    }
                }
            }
        }
    }

    let mut dp_map = BTreeMap::new();
    for dp_value in dp_vec {
        let dn = DeploymentNs {
            deploy: dp_value.deploy.clone(),
            ns: dp_value.ns.clone(),
        };

        if !dp_map.contains_key(&dn) {
            dp_map.insert(dn.clone(), dp_value);
        }
    }

    let mut result: Vec<DeploymentPod> = Vec::new();
    for (_, v) in dp_map {
        result.push(v.clone());
    }
    result
}
