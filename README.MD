# TODOs

- Make subject a one to many relationship from metadata for each one
- Two tables, subject en and subject ar
- Add migrations + entities for that
- Create a view which you will query; limit number of subjects on each record to like idk 200 and make sure you have
  this when writing to db; can't have more than 200 subjects on a given accession. This should be static in config.
- Then change filter to query the view versus what it is currently
- Add create subject endpoints and list subject endpoints for up to 200.

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