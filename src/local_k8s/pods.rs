use std::vec;

use k8s_openapi::api::core::v1::{Namespace, Pod};

use kube::{
    Client, ResourceExt,
    api::{Api, AttachParams, ListParams},
    core::ObjectList,
};
use tokio::io::AsyncReadExt;

pub async fn pods_exec_log(command: Vec<String>) -> anyhow::Result<(), anyhow::Error> {
    let (k8s_client, ns_all) = get_all_ns_resources().await;
    let lp_ns = ListParams::default();
    for ns in ns_all.list(&lp_ns).await? {
        let ns_name = ns.clone().metadata.name.unwrap();
        let pods: Api<Pod> = Api::namespaced(k8s_client.clone(), &ns_name);
        pod_exec(pods, command.clone()).await?
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

async fn pod_exec(pods: Api<Pod>, cmd: Vec<String>) -> anyhow::Result<(), anyhow::Error> {
    let lp_pod = ListParams::default();
    let pods_list: ObjectList<Pod> = pods.list(&lp_pod).await?;
    let mut bash_script = vec!["/bin/sh".to_string(), "-c".to_string()];

    bash_script.extend(cmd);

    for pod in pods_list {
        if let Some(container) = pod
            .spec
            .clone()
            .and_then(|spec| spec.containers.into_iter().find(|c| c.name == "app"))
        {
            if get_running_pod(&pod) {
                let ap = AttachParams {
                    stderr: true,
                    stdin: false,
                    stdout: true,
                    container: Some(container.name),
                    ..Default::default()
                };
                let mut attached = pods.exec(&pod.name_any(), bash_script.clone(), &ap).await?;
                let mut stdout_reader = attached.stdout().unwrap();
                let mut stderr_reader = attached.stderr().unwrap();

                let mut std_output = String::new();
                let mut err_output = String::new();
                stdout_reader.read_to_string(&mut std_output).await?;
                stderr_reader.read_to_string(&mut err_output).await?;

                if !std_output.is_empty() {
                    println!(
                        "ns={}, pod={}, stdout log:\n{}",
                        pod.namespace().unwrap(),
                        pod.name_any(),
                        std_output,
                    );
                }
                if !err_output.is_empty() {
                    println!(
                        "ns={}, pod={}, stderr log:\n{}",
                        pod.namespace().unwrap(),
                        pod.name_any(),
                        err_output,
                    );
                }
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
