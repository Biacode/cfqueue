# cqueue

An extremely simple in-memory job queue implementation.

Supported transports: **HTTP**

Supported storages: **In-Memory**

## Getting started

* Install [Rust toolchain](https://www.rust-lang.org/tools/install)
* Clone [cqueue repository](https://github.com/Biacode/cqueue) `git clone git@github.com:Biacode/cqueue.git`
* Execute `cargo run` command from within the git repo `cd cqueue && cargo run`

## Using `docker-compose`

Start your application by running

```shell
docker compose up --build
```

Output

```text
2024-05-14T11:33:31.678400Z  INFO cfqueue: 🚀🚀🚀CFQueue is up and running🚀🚀🚀
2024-05-14T11:33:31.678429Z  INFO cfqueue: Listening on [::1]:3000
```

Your application will be available at <http://localhost:3000>

You may also customize your container using environment variables.

## Configuring the server

You have two options to configure your server, using cli args and env vars.

### Using CLI

Use `cargo run -- --help` to see all available options

An example command might look like

```shell
cargo run -- --addr localhost --port 3000 --log-level info
```

### Using ENV Vars

Available env vars:

* CFQUEUE_ADDR - Server address.
* CFQUEUE_PORT - Server port.
* CFQUEUE_LOG_LEVEL - Root logging level.

## Known limitations

Currently, the queue supports only in-memory stores and HTTP as transport.

There is only an in-memory store, so your ID sequence will reset each time you start the app.

The app doesn't support deployments in replicated mode. Each server instance will maintain its ID sequence.
To solve this, we may consider introducing a "master" server using a simple consensus algorithm
like [Raft](https://en.wikipedia.org/wiki/Raft_(algorithm)) (No! thank you! not Paxos...)
or [Bully](https://en.wikipedia.org/wiki/Bully_algorithm).

The consumer (dequeue) acts as a basic request/response. We might consider switching to a more "interactive" mode. E.g.,
using WebSockets, SSE, Long-Pooling, etc.

## TODO

* [x] Implement in-memory queue store
* [x] Add HTTP server capabilities
* [x] Add docker support
* [ ] Observability/Monitoring solutions
* [ ] OpenAPI/Swagger support?
* [ ] Implement CI/CD pipelines using GitHub actions
* [x] Add command line arg parser to configure app properties. E.g, server port
* [x] The app should also be configurable with env variables (perhaps with higher priority?)
* [x] Improve documentation and add examples (partially done)
* [x] Changelog generator?
* [ ] Auth support
* [ ] Add docker publish to GitHub Actions
* [ ] REST API Rate limiter
* [x] Cancel job

## Developer notes

* Consumers could provide ID in headers, but I thought it could have been more fun. Also, the consumer may
  not know any ID, as we return ID only to the producer.
* As this is an in-memory store and everything will purged after restart, I don't care about keeping the ID
  sequence.
* Initially, I kept AtomicUsize for the ID counter, then changed it to the actual size of the `jobs` map.
* The fun part is we may discuss the "ID thing" for quite some time. Sharding/Partitioning, Replication, LB? Cool
  stuff!!
* To save some time, I borrowed an example server impl
  from [Axum examples](https://github.com/tokio-rs/axum/blob/main/examples/error-handling/src/main.rs)
* To save some time, I decided not to break modules into fine-grained modules or even submodules. This way, we might gain
benefits like restricting each module's dependencies, providing more encapsulation and fluent API, better maintainability, etc.
* Ideally, the Web layer should have been better decoupled from the persistence/repository layer. E.g., separating the
  structures by adding [DTO](https://en.wikipedia.org/wiki/Data_transfer_object) types for the representation layer,
  introducing some [CQRS](https://en.wikipedia.org/wiki/Command_Query_Responsibility_Segregation) concepts, etc.

## API

The queue exposes a REST API that producers and consumers perform HTTP requests against in JSON.

A typical scenario could look like

```text
/jobs/enqueue -> /jobs/dequeue -> /jobs/{job_id}/conclude
```

Supported operations:

### `/jobs/enqueue`

Add a job to the queue. The job definition can be found below.
Returns the ID of the job

cURL

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

cURL

```shell
curl --location --request POST 'localhost:3000/jobs/dequeue' \
--header 'Content-Type: application/json'
```

### `/jobs/{job_id}/conclude`

Provided an input of a job ID, finish execution on the job and consider it done

cURL

```shell
curl --location --request POST 'localhost:3000/jobs/conclude/1' \
--header 'Content-Type: application/json'
```

### `/jobs/{job_id}`

Given an input of a job ID, get information about a job tracked by the queue

The lifecycle of requests made for a job might look like this:

cURL

```shell
curl --location 'localhost:3000/jobs/1' \
--header 'Content-Type: application/json'
```

### `/jobs/stats`

Collect the current queue and job stats.

cURL

```shell
curl --location 'localhost:3000/jobs/stats' \
--header 'Content-Type: application/json'
```

### `/jobs/cancel`

Cancel the job by ID

cURL

```shell
curl --location --request POST 'localhost:3000/jobs/cancel' \
--header 'Content-Type: application/json' \
--data '{
    "ID": 1
}'
```

### Git cliff

```shell
cargo install git-cliff
```

Generate a changelog

```shell
git cliff -o --tag <your_tag> CHANGELOG.md
```
