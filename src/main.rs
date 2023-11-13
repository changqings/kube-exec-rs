use k8s_openapi::api::core::v1::{Namespace, Pod};

use kube::{
    api::{Api, AttachParams, ListParams},
    core::ObjectList,
    Client, ResourceExt,
};
use tokio::io::AsyncReadExt;

struct OsVersion {
    id: String,
    version: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // this ratelimit for check some error, not used
    // let config = Config::infer().await?;
    // let https = config.rustls_https_connector()?;
    // let service = ServiceBuilder::new()
    //     .layer(config.base_uri_layer())
    //     .layer(RateLimitLayer::new(5, Duration::from_secs(1)))
    //     .service(hyper::Client::builder().build(https));
    // let _k8s_client = Client::new(service, config.default_namespace);

    let k8s_client = Client::try_default().await?;

    let ns_all: Api<Namespace> = Api::all(k8s_client.clone());
    let lp = ListParams::default();
    for ns in ns_all.list(&lp).await? {
        let ns_name = ns.clone().metadata.name.unwrap();
        let pods: Api<Pod> = Api::namespaced(k8s_client.clone(), &ns_name);
        let pods_list: ObjectList<Pod> = pods.list(&lp).await?;
        for pod in pods_list {
            if let Some(container) = pod
                .spec
                .clone()
                .and_then(|spec| spec.containers.into_iter().find(|c| c.name == "app"))
            {
                if get_running_pod(pod.clone()) {
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
    }

    //

    Ok(())
}

fn get_running_pod(p: Pod) -> bool {
    let owner_ref = p.owner_references();

    if owner_ref.len() < 1 || owner_ref[0].kind == "Job".to_string() {
        return false;
    };

    if let Some(s) = p.status {
        if s.container_statuses.is_some() {
            for c in s.container_statuses.unwrap().iter() {
                if c.name == "app" && c.ready == true {
                    return true;
                }
            }
        }
    } else {
        return false;
    }

    return false;
}
