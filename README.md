# tuning [![badge](https://action-badges.now.sh/jokeyrhyme/tuning)](https://github.com/jokeyrhyme/tuning/actions)

ansible-like tool with a smaller scope, focused primarily on complementing dotfiles for cross-machine bliss

# status

- no functionality yet,
  collecting some thoughts together first

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

- read about the [jobs configuration file format](https://github.com/jokeyrhyme/tuning/wiki/Jobs-definition)

# roadmap

- [x] read config from user's HOME directory
- [x] support the "command" job
- [ ] support the "file" job
- [ ] support the "git" job
- [ ] flag to point at a different config file
- [ ] `import` or `include` to help decompose large config files
- [ ] `when` to support conditional execution of jobs
- [x] `needs` to support optional sequencing of jobs
