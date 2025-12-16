# TODOs

- Then implement the s3 upload part. Problems:
    - Remove unnecessary deps
    - Tidy up imports
    - Add tests
    - Figure out how to get JSON data through in the multipart form
    - Then write that JSON data to create an accession that points to s3 upload we just made
    - Make the filenames unique
    - Check you understand wtf is going on with boxing and readers and pinned futures and document it
    - Delete incomplete multipart uploads using s3cmd https://docs.digitalocean.com/products/spaces/how-to/configure-lifecycle-rules/#remove-incomplete-multipart-uploads
    - Should maybe put that in some kind of infra repo to track this stuff; put tickets like this there
    - Cargo clean and cargo update
    - Log in MB not bytes

# Overview

This is the API for the Sudan Digital Archive.

It is an axum web application that talks to a postgres database and
[browsertrix](https://browsertrix.com/). 

## Local Development 

You will need a local postgres database running. To run migrations, 

```shell
export DATABASE_URL="postgresql://<your connection string>"
sea-orm-cli migrate up
```

## Database

It's pretty useful to generate entities with sea orm. That way you
get a rust model for exactly what is in your database. After running
migrations, to generate entities:
```shell
sea-orm-cli generate entity -u $DATABASE_URL -o entity/src
```

The code generation typically doesn't work with the layout of the app - it writes
stuff into `mod.rs` not `lib.rs` so I would focus instead on using it just for 
modelling what's in the database, where it is perfect.

## Dockerfile

To test the Dockerfile, install docker, add the required environment variables to
`.env.local` and then run the compose file with 
`docker compose -f docker-compose.local.yml up`.

## Testing 

Just run `cargo test`. Note that clippy and tests run in CI on pull and merge
to main. So, both need to pass before releasing code.

Note that at the time of writing, there are no integration tests - only unit. 
You therefore should manually test anything that involves I/O to
external resources e.g. database since none of that functionality
will run itests.

## Deployment

Merging to main triggers a push of the container image to Digital Ocean registry.
This overwrites the most recent `latest` tag and will automatically trigger a
redeploy of the application to app platform.