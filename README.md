# kube-minion

A console application which is meant to be used with [minikube](https://minikube.sigs.k8s.io/docs/) and facilitates:

* Exposing the [Kubernetes](https://kubernetes.io/) dashboard through a load balancer service
* Starting the [minikube tunnel](https://minikube.sigs.k8s.io/docs/commands/tunnel/) in the background
* Setting up load balancers for exposing applications from inside a [Kubernetes](https://kubernetes.io/) cluster
* Setting up socat tunnels
* Mounting directories from [minikube](https://minikube.sigs.k8s.io/docs/)'s host onto the
  [minikube](https://minikube.sigs.k8s.io/docs/)'s filesystem

## Prerequisites

The following must be installed before using `kube-minion`.

* [Docker Engine](https://docs.docker.com/engine/install/)
* [minikube](https://minikube.sigs.k8s.io/docs/)
* [kubectl](https://kubernetes.io/docs/tasks/tools/install-kubectl-linux/)
* [socat](https://www.redhat.com/sysadmin/getting-started-socat)
* [SSH](https://www.ssh.com/academy/ssh)

## How-to

When starting, this application does the following:

1. Checks whether or not all the prerequisites are present
2. Creates a load balancer for the [Kubernetes](https://kubernetes.io/) dashboard, to be accessed at
   http://localhost:51515
3. Starts the [minikube tunnel](https://minikube.sigs.k8s.io/docs/commands/tunnel/)
4. Checks whether or not an initialization file (eg, `kube-minion.json`) is present
5. Starts the application's main loop

The main part of this application is basically a loop which executes the following steps:

1. Build a list of options for the user to choose from
2. Display these options
3. Let the user select an option
4. Execute the function that is related with the selected option

The options menu can be refreshed by selecting option `0`.

The application ignores `Ctrl-C` (`SIGINT`), so that the user has to explicitly choose whether to clean up or not
when exiting the application.

## Options

0. **Refresh options**
    * Start over from the top of the loop and rebuild the options list
1. **Create/Delete [Kubernetes](https://kubernetes.io/) dashboard load balancer**
    * Creates or deletes the load balancer that exposes the [Kubernetes](https://kubernetes.io/) dashboard at
      http://localhost:51515
2. **Start/Stop minikube tunnel**
    * Starts or stops the [minikube tunnel](https://minikube.sigs.k8s.io/docs/commands/tunnel/)
3. **Set the minikube tunnel bind address**
    * Sets the [minikube tunnel](https://minikube.sigs.k8s.io/docs/commands/tunnel/) bind address and restarts it if it
      is already started.
4. **Create load balancer**
    * Creates a [Kubernetes](https://kubernetes.io/) load balancer to expose an application
    * This requires the [minikube tunnel](https://minikube.sigs.k8s.io/docs/commands/tunnel/) to also be running for the
      application to become reachable
5. **List load balancers**
    * Lists the [Kubernetes](https://kubernetes.io/) load balancers that have been created by `kube-minion`
6. **Delete load balancer**
    * Deletes a [Kubernetes](https://kubernetes.io/) load balancer that has been created by `kube-minion`
7. **Delete all load balancers**
    * Deletes all [Kubernetes](https://kubernetes.io/) load balancers that have been created by `kube-minion`
8. **Create socat tunnel**
    * Creates a [socat](https://www.redhat.com/sysadmin/getting-started-socat) tunnel
    * This is useful when trying to access an application from the Windows environment while the application
      has been proxied inside WSL
9. **List socat tunnels**
    * Lists the [socat](https://www.redhat.com/sysadmin/getting-started-socat) tunnels that have been created by
      `kube-minion`
10. **Delete socat tunnel**
    * Deletes a [socat](https://www.redhat.com/sysadmin/getting-started-socat) tunnel that has been created by
      `kube-minion`
11. **Delete all socat tunnels**
    * Deletes all [socat](https://www.redhat.com/sysadmin/getting-started-socat) tunnels that have been created by
      `kube-minion`
12. **Set socat default connect host**
    * Sets the default [socat](https://www.redhat.com/sysadmin/getting-started-socat) connect host for connecting to the
     receiving end of the [socat](https://www.redhat.com/sysadmin/getting-started-socat) tunnel
    * By default, this is `localhost`
    * As an example, this can be used to configure `kube-minion` to use the required WSL interface by default
13. **Create minikube mount**
     * Creates a [minikube mount](https://minikube.sigs.k8s.io/docs/commands/mount/)
     * As an example, this can be used to mount a configuration or source code directory inside a pod
14. **List minikube mounts**
     * Lists the [minikube mount](https://minikube.sigs.k8s.io/docs/commands/mount/)s that have been created by
       `kube-minion`
15. **Delete minikube mount**
     * Deletes a [minikube mount](https://minikube.sigs.k8s.io/docs/commands/mount/) that has been created by
       `kube-minion`
16. **Delete all minikube mounts**
     * Deletes all [minikube mount](https://minikube.sigs.k8s.io/docs/commands/mount/)s that have been created by
       `kube-minion`
17. **Clean up and exit**
    * Deletes the load balancer that exposes the [Kubernetes](https://kubernetes.io/) dashboard at
      http://localhost:51515
    * Deletes all [Kubernetes](https://kubernetes.io/) load balancers that have been created by `kube-minion`
    * Deletes all [socat](https://www.redhat.com/sysadmin/getting-started-socat) tunnels that have been created by
      `kube-minion`
    * Deletes all [minikube mount](https://minikube.sigs.k8s.io/docs/commands/mount/)s that have been created by
      `kube-minion`
    * Exits the application
18. **Exit without cleaning up**
    * Exits the application without the cleaning up done by **Clean up and exit**
19. **Clean up initialization file configuration and exit** (available only when an
    [initialization file](#configuration) has been found)
    * Undoes all the configuration that has been specified in the found [initialization file](#configuration)

## Configuration

To see the available command line options, use `kube-minion -h`.

The `-p | --dashboard-port` command line parameter sets the port on which to expose the
[Kubernetes](https://kubernetes.io/) dashboard load balancer service.

The `-f | --initialization-file-path` command line parameter sets the initialization file from which to read initial
configuration. When used, the [KUBE_MINION_ENVIRONMENT](#environment-variable-kube_minion_environment) environment
variable is not considered.

Additionally, the application can be configured with an initialization file.

This file is a JSON file for which, a
[schema](https://raw.githubusercontent.com/sadesyllas/kube-minion/main/kube-minion.schema.json)
has been provided.

This schema exists in file `kube-minion.schema.json` and an exhaustive template
exists in file `kube-minion.template.json`.

#### Initialization file

The initialization file is expected to be found in the same directory as the one in which
`kube-minion` has been started in.

This file can configure the following:

* Load balancers to expose applications that run in [minikube](https://minikube.sigs.k8s.io/docs/)'s
  [Kubernetes](https://kubernetes.io/) cluster.
* [socat](https://www.redhat.com/sysadmin/getting-started-socat) tunnels
* The default [socat](https://www.redhat.com/sysadmin/getting-started-socat) connect host at the tunnel's receiving end
* [minikube mount](https://minikube.sigs.k8s.io/docs/commands/mount/)s

#### Environment variable `KUBE_MINION_ENVIRONMENT`

This environment variable makes `kube-minion` search for an initialization file with a name of
`kube-minion.$KUBE_MINION_ENVIRONMENT.json`, instead of the default name of `kube-minion.json`.
