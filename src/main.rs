use std::{collections::BTreeMap, time::Duration};

use futures::StreamExt;
use k8s_openapi::api::core::v1::{Namespace, Pod, PodStatus};

use kube::{
    api::{Api, AttachParams, AttachedProcess, ListParams},
    core::ObjectList,
    Client, ResourceExt,
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
    let ns_all: Api<Namespace> = Api::all(Client::try_default().await?);
    let lp = ListParams::default();
    for ns in ns_all.list(&lp).await? {
        let ns_name = ns.clone().metadata.name.unwrap();
        let pods: Api<Pod> = Api::namespaced(Client::try_default().await?, &ns_name);
        let pods_list: ObjectList<Pod> = pods.list(&lp).await?;
        for pod in pods_list {
            if let Some(container) = pod
                .spec
                .clone()
                .and_then(|spec| spec.containers.into_iter().find(|c| c.name == "app"))
            {
                // if get_running_pod(&pod.status.as_ref().unwrap()) {
                let cmd = vec!["cat", "/etc/os-release"];
                let attached: AttachedProcess = pods
                    .exec(
                        pod.name_any().as_ref(),
                        cmd,
                        &AttachParams::default().container(container.name),
                    )
                    .await?;
                // attached.abort();
                // println!("next");
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
                    ns.name_any(),
                    pod.name_any(),
                    os.id,
                    os.version
                );
            }
        }
        let _ = tokio::time::sleep(Duration::from_secs(1));
    }

    //

    Ok(())
}

async fn get_output(mut attached: AttachedProcess) -> String {
    let stdout = tokio_util::io::ReaderStream::new(attached.stdout().unwrap());
    let out = stdout
        .filter_map(|r| async { r.ok().and_then(|v| String::from_utf8(v.to_vec()).ok()) })
        .collect::<Vec<_>>()
        .await
        .join("");
    attached.join().await.unwrap();
    out
}

fn get_running_pod(p: &PodStatus) -> bool {
    if let Some(pc) = p.clone().conditions {
        for st in pc {
            if st.type_ == "Ready" {
                return true;
            }
        }
    }
    false
}

// generate by gpt4
fn _get_deploy_one_pod(pod_list: &ObjectList<Pod>) -> Vec<DeploymentPod> {
    let mut dp_vec: Vec<DeploymentPod> = Vec::new();

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

                        if get_running_pod(&pod.status.as_ref().unwrap()) {
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
