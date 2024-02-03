use k8s_openapi::api::core::v1::{Namespace, Pod};

use kube::{
    api::{Api, AttachParams, ListParams},
    core::ObjectList,
    Client, ResourceExt,
};
use tokio::io::AsyncReadExt;

#[derive(Debug)]
pub struct OsVersion {
    pub id: String,
    pub version: String,
}

pub async fn print_pods_log() -> anyhow::Result<()> {
    let (k8s_client, ns_all) = get_all_ns_resources().await;
    let lp_ns = ListParams::default();
    for ns in ns_all.list(&lp_ns).await? {
        let ns_name = ns.clone().metadata.name.unwrap();
        let pods: Api<Pod> = Api::namespaced(k8s_client.clone(), &ns_name);
        pod_exec(pods, ns).await?
    }
    Ok(())
}

async fn get_all_ns_resources() -> (Client, Api<Namespace>) {
    // this ratelimit for check some error, not used
    // let config = Config::infer().await?;
    // let https = config.rustls_https_connector()?;
    // let service = ServiceBuilder::new()
    //     .layer(config.base_uri_layer())
    //     .layer(RateLimitLayer::new(5, Duration::from_secs(1)))
    //     .service(hyper::Client::builder().build(https));
    // let _k8s_client = Client::new(service, config.default_namespace);

    let client = Client::try_default().await.unwrap();
    return (client.clone(), Api::all(client));
}

async fn pod_exec(pods: Api<Pod>, ns: Namespace) -> anyhow::Result<()> {
    let lp_pod = ListParams::default();
    let pods_list: ObjectList<Pod> = pods.list(&lp_pod).await?;

    for pod in pods_list {
        if let Some(container) = pod
            .spec
            .clone()
            .and_then(|spec| spec.containers.into_iter().find(|c| c.name == "app"))
        {
            if get_running_pod(&pod) {
                let ap = AttachParams {
                    stderr: false,
                    stdin: true,
                    stdout: true,
                    max_stdin_buf_size: Some(1024 * 1024),
                    max_stdout_buf_size: Some(1024 * 1024 * 1024),
                    container: Some(container.name),
                    ..Default::default()
                };
                let cmd = vec!["cat", "/etc/os-release"];
                let mut attached = pods.exec(&pod.name_any(), cmd, &ap).await?;
                let mut stdout_reader = attached.stdout().unwrap();
                let mut output = String::new();
                stdout_reader.read_to_string(&mut output).await?;

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
    }
    Ok(())
}

fn get_running_pod(p: &Pod) -> bool {
    let owner_ref = p.owner_references();

    if owner_ref.len() < 1 || owner_ref[0].kind == "Job".to_string() {
        return false;
    };

    if let Some(s) = &p.status {
        if s.container_statuses.is_some() {
            for c in s.container_statuses.as_ref().unwrap().iter() {
                if c.name == "app" && c.ready == true {
                    return true;
                }
            }
        }
    }

    return false;
}
