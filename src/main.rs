use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::apps::v1::DaemonSet;
use k8s_openapi::api::core::v1::ConfigMap;
use k8s_openapi::api::core::v1::Pod;
use kube::api::{Api, ListParams, PostParams, WatchEvent};
use kube::runtime::Informer;
use kube::Client;

#[tokio::main]
async fn main() -> Result<(), kube::Error> {
    // Read the environment to find config for kube client.
    // Note that this tries an in-cluster configuration first,
    // then falls back on a kubeconfig file.
    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());

    let data = r#"
        sources:
        - id: system-auth
        type: file
        file_pattern: /var/log/auth\.log(\.(?P<rotation>\d)(\.gz)?)?
        line_pattern: "%{SYSLOGTIMESTAMP:timestamp} %{GREEDYDATA:message}"
        datetime_pattern: "%b %d %H:%M:%S"
        timezone: "Europe/Berlin"
        - id: system-syslog
        type: file
        file_pattern: /var/log/syslog(\.\d(\.gz)?)?
        line_pattern: "%{SYSLOGTIMESTAMP:timestamp} %{GREEDYDATA:message}"
        datetime_pattern: "%b %d %H:%M:%S"
        timezone: "Europe/Berlin"
        - id: system-sshd
        type: journal
        unit: sshd
        line_pattern: "%{SYSLOGTIMESTAMP:timestamp} %{GREEDYDATA:message}"
        datetime_pattern: "%b %d %H:%M:%S"
        timezone: "Europe/Berlin"
      "#;

    let configmap_spec = serde_json::from_value(serde_json::json!({
            "kind": "ConfigMap",
            "apiVersion": "v1",
            "metadata": {
                "name": "logtopus-tentacle-config",
            },
            "data": {
                "ubuntu.yml": &data
            }
        }
    ))?;

    let daemonset_spec = serde_json::from_value(serde_json::json!({
            "apiVersion": "apps/v1",
            "kind": "DaemonSet",
            "metadata": {
                "name": "logtopus-tentacle",
                "namespace": &namespace
            },
            "spec" : {
                "selector" : {
                    "matchLabels" : {
                        "name": "logtopus-tentacle"
                    }
                },
                "template" : {
                    "metadata" : {
                        "labels" : {
                            "name": "logtopus-tentacle"
                        }
                    },
                    "spec": {
                        "tolerations" : [ {
                            "key": "node-role.kubernetes.io/master",
                            "effect": "NoSchedule"
                        }],
                        "containers": [{
                                "name": "logtopus-tentacle",
                                "image": "hello-world:latest",
                                "volumeMounts" : [{
                                "name": "config",
                                "mountPath": "/etc/logtopus/tentacle.conf"
                                }]
                        }],
                        "volumes": [{
                            "name": "config",
                            "configMap" : {
                                "name": "logtopus-tentacle-config"
                            }
                        }]
                    }
                }
            }
        }
    ))?;

    let client = Client::try_default().await?;
    let api: Api<ConfigMap> = Api::namespaced(client, &namespace);

    let _config_map = api.create(&PostParams::default(), &configmap_spec).await?;

    let client = Client::try_default().await?;
    let api: Api<DaemonSet> = Api::namespaced(client, &namespace);

    let _daemon_set = api.create(&PostParams::default(), &daemonset_spec).await?;

    let client = Client::try_default().await?;
    let api: Api<Pod> = Api::namespaced(client, &namespace);

    let informer = Informer::new(api).params(
        ListParams::default()
            .fields("metadata.name=logtopus-tentacle")
            .timeout(120),
    );

    // Get an event stream from the informer
    let mut events_stream = informer.poll().await?.boxed();

    // Keep getting events from the events stream
    while let Some(event) = events_stream.try_next().await? {
        match event {
            WatchEvent::Modified(e)
                if e.status.as_ref().unwrap().phase.as_ref().unwrap() == "Running" =>
            {
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
