use futures::{StreamExt, TryStreamExt};
use kube::api::{Api, ListParams, PostParams, WatchEvent};
use kube::Client;
use kube::runtime::Informer;
use k8s_openapi::api::core::v1::Pod;

#[tokio::main]
async fn main() -> Result<(), kube::Error> {
    // Read the environment to find config for kube client.
    // Note that this tries an in-cluster configuration first,
    // then falls back on a kubeconfig file.
    let kube_client = Client::try_default().await?;

    // Get a strongly typed handle to the Kubernetes API for interacting
    // with pods in the "default" namespace.
    let pods: Api<Pod> = Api::namespaced(kube_client, "default");

    // Create a pod from JSON
    let pod = serde_json::from_value(serde_json::json!({
        "apiVersion": "v1",
        "kind": "Pod",
        "metadata": {
            "name": "my-pod"
        },
        "spec": {
            "containers": [
                {
                    "name": "my-container",
                    "image": "hello-world:latest",
                },
            ],
        }
    }))?;

    // Create the pod
    let pod = pods.create(&PostParams::default(), &pod).await?;

    // Create an informer for watching events about
    let informer = Informer::new(pods).params(
        ListParams::default()
            .fields("metadata.name=my-pod")
            .timeout(120),
    );

    // Get an event stream from the informer
    let mut events_stream = informer.poll().await?.boxed();

    // Keep getting events from the events stream
    while let Some(event) = events_stream.try_next().await? {
        match event {
            WatchEvent::Modified(e) if e.status.as_ref().unwrap().phase.as_ref().unwrap() == "Running" => {
                println!("It's running!");
            }
            WatchEvent::Error(e) => {
                panic!("WatchEvent error: {:?}", e);
            }
            _ => {}
        }
    }
    Ok(())
}