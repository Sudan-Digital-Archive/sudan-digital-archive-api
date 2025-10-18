# To Dos

## Flow

- Download from browsertrix
- Upload to s3 bucket
- Get s3 file id
- Write that into accession record
- Read from s3 not btrix
- Write tests

## Changes

- Create s3 storage in DO and init client w/access keys
- Add backlog tickets to delete data from btrix, and backfill 
  the s3 filenames for older records


## Notes

- https://docs.digitalocean.com/products/spaces/how-to/use-aws-sdks/
- There is a delete crawl endpoint https://app.browsertrix.com/api/docs#/ but not really
  worth it since crawl time unlikely to exceed storage. However make a to do that 
  describes this
- Then just download & upload file - there is a download wacz endpoint in browsertrix. Need to keep an eye on memory for that...
  2GB in the file system https://docs.digitalocean.com/products/app-platform/details/limits/; 10GiB of memory.
  Probably fine to do this in memory up to 5GB. Mabye worth implementing a repo just for async tasks so these can get moved somewhere sensible later.
  [Examples](https://docs.aws.amazon.com/sdk-for-rust/latest/dg/rust_s3_code_examples.html)
- Then add the spaces object id to database
- Then remove the enrich with browsertrix stuff instead just get a temporary authenticated url from s3; pre-sign for 1h
- Read up on unauthenticated requests stuff - just set
  [these](https://www.digitalocean.com/security/security-best-practices-guide-spaces) security things and serve over CDN
- Make sure to turn on digital ocean cdn for serving the archive file too, think that will be cheaper
- Document max crawl size in browsertrix; I think it's 1gb atm which should be fine? Unlikely to hit that atm, can probably tweak settings
  if we ever get there later. Just log this well and how much data you are streaming each time.
- Then gonna need to do a one off script to txfer all the older records into spaces. That is going to need an API key + a put endpoint
  that supports editing the wacz url if you are admin only. Sigh, can do that later.
- Also tighten up the rate limiting.

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