# tuning [![badge](https://action-badges.now.sh/jokeyrhyme/tuning)](https://github.com/jokeyrhyme/tuning/actions)

ansible-like tool with a smaller scope, focused primarily on complementing dotfiles for cross-machine bliss

MOVED: https://gitlab.com/jokeyrhyme/tuning

# status

- some functionality,
  but still missing some basics that will make it actually useful

# what?

- inspired by [`ansible`](https://www.ansible.com/),
  with jobs defined in a declarative file format

- will focus on the dotfiles use-case:
  maintaining the same user preferences across machines,
  or restoring user preferences to a fresh machine

- no remote or fleet functionality / complexity

- not aiming to do anything that requires root / Administrator permissions (yet)

- named after the reality-bending ability in [Dark City](https://en.wikipedia.org/wiki/Dark_City_%281998_film%29)

# why?

- my dotfiles started as a [whole bunch of shell-scripts](https://github.com/jokeyrhyme/dotfiles),
  but didn't have any support for Windows,

- I'd partially moved to [my second attempt](https://github.com/jokeyrhyme/dotfiles-rs),
  which is cross-platform,
  but required too much work for new jobs

- other existing tools use interpretted languages,
  which are fine for web services that run in containers,
  but can be overly-sensitive to interpreter versions and globally-installed packages

- yes, I am firmly trapped in [The Code/Data Cycle](https://twitter.com/niklasfrykholm/status/1063242674717679621)

# prerequisites

- Rust compiler and `cargo`: https://rustup.rs/

# getting started

```
$ cargo install tuning
$ tuning
```

# documentation

- read about [job metadata](./docs/metadata.md)
- read about [job file template rendering](./docs/template.md)
- read about the [jobs configuration file format](https://github.com/jokeyrhyme/tuning/wiki/Jobs-definition)

# roadmap

- [x] read config from user's HOME directory
- [x] `needs` to support optional sequencing of jobs
- [x] support the "command" job
- [x] support the "file" job
- [x] resolve references to path expressions (e.g. ~) ([#9](https://github.com/jokeyrhyme/tuning/issues/9))
- [x] `when` to support conditional jobs
- [x] specify that a job needs a certain OS
- [x] specify that a job needs certain executables
- [ ] `needs_any` for flexible sequencing of jobs
- [ ] support the "git" job
- [ ] flag to point at a different config file
- [ ] `import` or `include` to help decompose large config files

# see also

- https://github.com/rash-sh/rash
- https://www.ansible.com/
