# cqueue

An extremely simple in-memory job queue implementation.

Supported transports: **HTTP**<br>
Supported storages: **In-Memory**

# Getting started

* Install [Rust toolchain](https://www.rust-lang.org/tools/install)
* Clone [cqueue repository](https://github.com/Biacode/cqueue) `git clone git@github.com:Biacode/cqueue.git`
* Execute `cargo run` command from within the git repo `cd cqueue && cargo run`

## Using `docker-compose`

Start your application by running

```shell
docker compose up --build
```

Your application will be available at http://localhost:3000

You may also customize your container using environment variables.

## Configuring the server

You have two options to configure your server, using cli args and env vars.

### Using CLI

Use `cargo run -- --help` to see all available options

An example command might look like

```shell
cargo run -- --addr localhost --port 3000 --log-level info
```

Output

```text
2024-05-14T11:33:31.678400Z  INFO cfqueue: CFQueue is up and running 🎉🎉🎉🚀🚀🚀
2024-05-14T11:33:31.678429Z  INFO cfqueue: Listening on [::1]:3000
```

### Using ENV Vars

Available env vars:

* CFQUEUE_ADDR - Server address.
* CFQUEUE_PORT - Server port.
* CFQUEUE_LOG_LEVEL - Root logging level.

# Known limitations

At the moment the queue supports only in-memory store and HTTP as transport.

As there is only in-memory store, your ID sequence will reset each time your start the app.

The app doesn't support deployments in replicated mode. Each server instance will maintain its own ID sequence.
To solve this we may consider introducing "master" server using simple consensus algorithm
like [Raft](https://en.wikipedia.org/wiki/Raft_(algorithm)) (No! thank you! not Paxos...)
or [Bully](https://en.wikipedia.org/wiki/Bully_algorithm).

The consumer (dequeue) acts as a basic request/response. We might consider switching to more "interactive" mode. E.g,
using WebSockets, SSE, Long-Pooling, etc.

# TODO;

* [x] Implement in-memory queue store
* [x] Add HTTP server capabilities
* [x] Add docker support
* [ ] Observability/Monitoring solutions
* [ ] OpenAPI/Swagger support?
* [ ] Implement CI/CD pipelines using GitHub actions
* [x] Add command line arg parser to configure app properties. E.g, server port
* [x] The app should be configurable with env variables as well (perhaps with higher priority?)
* [x] Improve documentation and add examples (partially done)
* [ ] Changelog generator?
* [ ] Auth support
* [ ] Add docker publish to GitHub Actions

# Developer notes

* I know the requirement says consumer provides ID in headers, but I thought it isn't much fun. Also, the consumer may
  not know any ID, as we return ID only to producer.
* As this is an in-memory store and everything will purged after restart, I don't really care about keeping the ID
  sequence.
* At first, I was keeping AtomicUsize for ID counter, then I changed it to actual size of the `jobs` map.
* The fun part is, we may discuss the "ID thing" for quite some time. Sharding/Partitioning, Replication, LB? cool
  stuff!!
* To save some time I borrowed an example server impl
  from [Axum examples](https://github.com/tokio-rs/axum/blob/main/examples/error-handling/src/main.rs)
* To save some time I decided not to break modules into fine-grained modules or even submodules. This way we might gain
  some benefits like restricting dependencies for each module, more encapsulation and fluent API, better
  maintainability, etc.
* Ideally, the Web layer should have been better decoupled from persistence/repository layer. E.g, separating the
  structures by adding [DTO](https://en.wikipedia.org/wiki/Data_transfer_object) types for representation layer,
  introducing some [CQRS](https://en.wikipedia.org/wiki/Command_Query_Responsibility_Segregation) concepts, etc.

# API

The queue exposes a REST API that producers and consumers perform HTTP requests against in JSON.

Supported operations:

### `/jobs/enqueue`

Add a job to the queue. The job definition can be found below.
Returns the ID of the job

**cURL**

```shell
curl --location --request PUT 'localhost:3000/jobs/enqueue' \
--header 'Content-Type: application/json' \
--data '{
    "Type": "TIME_CRITICAL"
}'
```

### `/jobs/dequeue`

Returns a job from the queue
Jobs are considered available for Dequeue if the job has not been concluded and has not dequeued already

**cURL**

```shell
curl --location --request POST 'localhost:3000/jobs/dequeue' \
--header 'Content-Type: application/json'
```

### `/jobs/{job_id}/conclude`

Provided an input of a job ID, finish execution on the job and consider it done

**cURL**

```shell
curl --location --request POST 'localhost:3000/jobs/conclude/1' \
--header 'Content-Type: application/json'
```

### `/jobs/{job_id}`

Given an input of a job ID, get information about a job tracked by the queue

The lifecycle of requests made for a job might look like this:

**cURL**

```shell
curl --location 'localhost:3000/jobs/1' \
--header 'Content-Type: application/json'
```

### `/jobs/stats`

Collect the current queue and job stats. The output is a tuple with the following
format (`<queue len>`, `<queued jobs>`, `<in progress jobs>`, `<concluded jobs>`).
The method is not included in public API as it is used for only debugging purpose.

**cURL**

```shell
curl --location 'localhost:3000/jobs/stats' \
--header 'Content-Type: application/json'
```

`/jobs/enqueue -> /jobs/dequeue -> /jobs/{job_id}/conclude`
