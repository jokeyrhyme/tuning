# job metadata

these are fields that are not specific to the job type,
but rather relate to running the job

## name (string, optional)

set a unique name / label / description / identifier for the job,
which appears in logs when the job runs

e.g.

```
[[jobs]]
name = "something to do"
# ...
```

## needs (string[], optional)

set dependencies for the job,
which **all** need to complete without errors,
before this job can run

e.g.

```
[[jobs]]
name = "first thing"
# ...

[[jobs]]
name = "second thing"
# ...
needs = ["first thing"]
```

## when (boolean; default = true)

e.g.

```
[[jobs]]
name = "something to do"
# ...
when = true
```

- `true`: run the job
- `false`: skip the job

this makes the most sense when combined with a boolean
[template expression](./template.md)

e.g.

```
[[jobs]]
name = "something to do"
# ...
when = {{ is_os_linux or is_os_macos }}
```
