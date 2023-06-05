use futures::{Stream, StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::Pod;

use kube::{
    api::{
        Api, AttachParams, AttachedProcess, DeleteParams, Execute, ListParams, Object, PostParams,
        ResourceExt, WatchEvent, WatchParams,
    },
    runtime::reflector::ObjectRef,
    Client,
};
use tokio::io::AsyncWriteExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client: Client = Client::try_default().await?;

    // let p: Pod = serde_json::from_value(serde_json::json!({
    //     "apiVersion": "v1",
    //     "kind": "Pod",
    //     "metadata": { "name": "example" },
    //     "spec": {
    //         "containers": [{
    //             "name": "example",
    //             "image": "alpine",
    //             // Do nothing
    //             "command": ["tail", "-f", "/dev/null"],
    //         }],
    //     }
    // }))?;

    let pods_all_ns: Api<Pod> = Api::all(client.clone());
    // Stop on error including a pod already exists or is still being deleted.
    // pods.create(&PostParams::default(), &p).await?;

    //
    let list_params: ListParams = ListParams::default().labels("");
    //
    let pod_list = pods_all_ns.list(&list_params).await?;

    for pod in pod_list {
        let name = pod.metadata.name.as_ref().unwrap();
        let ns = pod.metadata.namespace.as_ref().unwrap();

        if ns == "default" {
            let kube_pod: Api<Pod> = Api::namespaced(client.clone(), ns.as_str());

            let attached = kube_pod
                .exec(
                    &name,
                    vec!["cat", "/etc/os-release"],
                    &AttachParams::default().stderr(false),
                )
                .await?;

            let output = get_output(attached).await;
            println!("{output}");
        }

        // {
        //     let attached: AttachedProcess = pods
        //         .exec(
        //             "example",
        //             vec!["sh", "-c", "for i in $(seq 1 3); do date; done"],
        //             &AttachParams::default().stderr(false),
        //         )
        //         .await?;
        //     let output: String = get_output(attached).await;
        //     println!("{output}");
        //     assert_eq!(output.lines().count(), 3);
        // }

        // {
        //     let attached: AttachedProcess = pods
        //         .exec(
        //             "example",
        //             vec!["uptime"],
        //             &AttachParams::default().stderr(false),
        //         )
        //         .await?;
        //     let output: String = get_output(attached).await;
        //     println!("{output}");
        //     assert_eq!(output.lines().count(), 1);
        // }

        // // Stdin example
        // {
        //     let mut attached: AttachedProcess = pods
        //         .exec(
        //             "example",
        //             vec!["sh"],
        //             &AttachParams::default().stdin(true).stderr(false),
        //         )
        //         .await?;
        //     let mut stdin_writer = attached.stdin().unwrap();
        //     let mut stdout_stream = tokio_util::io::ReaderStream::new(attached.stdout().unwrap());
        //     let next_stdout = stdout_stream.next();
        //     stdin_writer.write_all(b"echo test string 1\n").await?;
        //     let stdout: String =
        //         String::from_utf8(next_stdout.await.unwrap().unwrap().to_vec()).unwrap();
        //     println!("{stdout}");
        //     assert_eq!(stdout, "test string 1\n");

        //     // AttachedProcess provides access to a future that resolves with a status object.
        //     let status = attached.take_status().unwrap();
        //     // Send `exit 1` to get a failure status.
        //     stdin_writer.write_all(b"exit 1\n").await?;
        //     if let Some(status) = status.await {
        //         println!("{status:?}");
        //         assert_eq!(status.status, Some("Failure".to_owned()));
        //         assert_eq!(status.reason, Some("NonZeroExitCode".to_owned()));
        //     }
        // }
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
