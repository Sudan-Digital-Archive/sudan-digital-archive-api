# Sudan Digital Archive API

## Important - Workflows

When working on this repo, you should make use of the fact that it's in Rust! You should always run 
`cargo check` after doing some changes to make sure that stuff compiles. Once you've done that, you should
run `export JWT_SECRET="test" && cargo test` to make sure you haven't broken the tests either.

## Project Overview

This project is the backend API for the Sudan Digital Archive. 
It's a Rust-based web application built with the `axum` framework. It uses a PostgreSQL database for data storage, 
`sea-orm` as an ORM, and integrates with `browsertrix` for web crawling and DigitalOcean Spaces for file storage. 
The API provides endpoints for managing accessions (archival records), subjects (metadata tags), 
and user authentication.

The API is designed to be documented with OpenAPI, using the `utoipa` crate to generate a Swagger UI.

## Building and Running

### Prerequisites

- Rust
- Docker
- A `.env` file with the necessary environment variables (see `README.MD` for an example).

### Local Development

1.  **Start the database:**
    ```shell
    docker compose -f docker-compose.local.yml up
    ```

2.  **Run database migrations:**
    ```shell
    export DATABASE_URL="postgresql://archivist:test@localhost/sudan_archives"
    cd migration
    cargo run -- up
    ```

3.  **Run the application:**
    ```shell
    export $(cat .env | xargs)
    cargo run
    ```

The application will be available at the address specified in the `LISTENER_ADDRESS` environment variable. 
The OpenAPI documentation can be accessed at `/sda-api/docs`.

### Testing

To run the tests, execute the following command:

```shell
export JWT_SECRET="some string" && cargo test
```

## Development Conventions

### Code Style

The project follows standard Rust conventions. `clippy` is used for linting and is run as part of the CI pipeline.

### Testing

The project has unit tests for its routes and services. These tests use in-memory repositories to avoid I/O operations. 

### API Documentation

The API is documented using the OpenAPI standard. The `utoipa` crate is used to generate the OpenAPI specification 
from the code. The documentation is available at the `/sda-api/docs` endpoint.

### CI/CD

The project has a CI/CD pipeline configured with GitHub Actions. The pipeline runs tests and `clippy` on every pull 
request and merge to the `main` branch. Merging to `main` also triggers a deployment to DigitalOcean App Platform.

## Key Files

-   `src/main.rs`: The application's entry point. It initializes the services, repositories, and the `axum` server.
-   `src/app_factory.rs`: Creates and configures the `axum` application, including middleware and routes.
-   `src/routes/`: This directory contains the route handlers for the different API resources:
    -   `accessions.rs`: Routes for managing accessions.
    -   `auth.rs`: Routes for user authentication.
    -   `subjects.rs`: Routes for managing subjects.
-   `src/services/`: This directory contains the business logic for the application.
-   `src/repos/`: This directory contains the database and external service repositories.
-   `entity/`: This directory contains the `sea-orm` entities that map to the database tables.
-   `migration/`: This directory contains the database migrations.
-   `Cargo.toml`: The project's manifest file, which lists the dependencies and other metadata.
-   `README.MD`: The project's main documentation, with detailed instructions for setting up and running the 
     application.
