# TODOs

- Change read + list accessions to hit the view
- Write tests for all the new shiz
- Write ticket to do open api schema

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

To then generate entities:
```shell
sea-orm-cli generate entity -u $DATABASE_URL -o entity/src
```

The code generation is buggy so don't rely on it. 
However, it provides a good starting point. I would recommend saving the
existing entity files before you do this so that you can see what a working version 
looks like.

You then need to set a lot of environment variables - take a look in the app config
struct for more info. After that's done, it's just `cargo run`.

## Dockerfile

To test the Dockerfile, install docker, add the required environment variables to
`.env.local` and then run the compose file with 
`docker compose -f docker-compose.local.yml up`.

## Testing 

Just run `cargo test`. Note that clippy and tests run in CI on pull and merge
to main. So, both need to pass before releasing code.

## Deployment

Merging to main triggers a push of the container image to Digital Ocean registry.
This overwrites the most recent `latest` tag and will automatically trigger a
redeploy of the application to app platform.