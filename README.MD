# TODOs

- Add tests with appropriate mock repo implementations
- Create a ticket in the infra repo to update the database diagram too
- Update docs to remove redoc

# Overview

This is the API for the Sudan Digital Archive.

It is an axum web application that talks to a postgres database and
[browsertrix](https://browsertrix.com/). 

## Local Development 

You will need a local postgres database running. You can use the `docker-compose.local.yml` file for this. Install
[docker](https://www.docker.com/) and then run `docker compose -f docker-compose.local.yml up`.

## Running the Application

Create a `.env` file with the required environment variables. Then:

```shell
export $(cat .env | xargs)
cargo run
```

Here is an exmple `.env` file:

```
POSTGRES_URL='<postgres connection string>'
ARCHIVE_SENDER_EMAIL='<email>'
POSTMARK_API_BASE='<api base>'
POSTMARK_API_KEY='<api key>'
BROWSERTRIX_USERNAME='<username>'
BROWSERTRIX_PASSWORD='<password>'
BROWSERTRIX_ORGID='<org id>'
BROWSERTRIX_BROWSERTRIX_URL='<api base>'
JWT_COOKIE_DOMAIN='<domain>'
CORS_URL='<cors url>'
LISTENER_ADDRESS='<api url>'
JWT_EXPIRY_HOURS='<expiry hours>'
JWT_SECRET="<secret>"
DO_SPACES_ENDPOINT_URL="<url>"
DO_SPACES_REGION="<region>"
DO_SPACES_BUCKET="<bucket name>"
DO_SPACES_ACCESS_KEY="<key>"
DO_SPACES_SECRET_KEY="<secret>"
```
Once the application is running, you can access swagger docs at `localhost:port/sda-api/docs`. The `sda-api` prefix is
there since it gets deployed to this prefix on Digital Ocean, however note that you can toggle to a local server in
Swagger so the requests go through without the prefix, which is required for local development.

## Migrations
 To run migrations:

```shell
export DATABASE_URL="postgresql://archivist:test@localhost/sudan_archives"
cd migration
cargo run -- up
```

## Database

It's pretty useful to generate entities with sea orm. That way you
get a rust model for exactly what is in your database. After running
migrations, to generate entities:
```shell
cargo run -- generate entity -u $DATABASE_URL -o entity/src
```

The code generation typically doesn't work with the layout of the app - it writes
stuff into `mod.rs` not `lib.rs` so I would focus instead on using it just for 
modelling what's in the database, where it is perfect.

## Dockerfile

To test the Dockerfile, install [docker](https://www.docker.com/) and then run `docker build .`.

## Testing 

Just run `export JWT_SECRET="some string" && cargo test`. Note that clippy and tests run in CI on pull and merge
to main. So, both need to pass before releasing code.

Note that at the time of writing, there are no integration tests - only unit. 
You therefore should manually test anything that involves I/O to
external resources e.g. database since none of that functionality
will run itests.

## Deployment

Merging to main triggers a push of the container image to Digital Ocean registry.
This overwrites the most recent `latest` tag and will automatically trigger a
redeploy of the application to app platform.