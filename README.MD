# Local dev

```shell
/usr/local/Cellar/postgresql@17/17.2/bin/psql postgres
```
To run migrations:
```shell
sea-orm-cli migrate up
```

To then generate entities:
```shell
sea-orm-cli generate entity -u $DATABASE_URL -o entity/src
```

This is a lot buggy - you need to tweak `mod.rs` to `lib.rs`. Would
just look at the model diffs. Also doesn't get 1-1 relationship right or handle nullability.
May be faster to do manually.

# TODOs

- Ship it 
