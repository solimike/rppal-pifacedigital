# Changelog

All notable changes to this project will be documented in this file.

Note that this file is auto-generated from [Conventional Commit](https://www.conventionalcommits.org/en/v1.0.0/)
formatted messages in the Git commit history.

## [0.1.0] - 2024-08-19

[b02f0ca](https://github.com/solimike/rppal-pifacedigital/commit/b02f0ca114a4fb168c3bf333ff7f8bab7ea4b736)...[fb663f5](https://github.com/solimike/rppal-pifacedigital/commit/fb663f580dcd1d9459f4afff70737d8c4dbf9de5)

### Features

- Support Raspberry Pi 5. ([c005210](https://github.com/solimike/rppal-pifacedigital/commit/c0052105f002e3342b9e39e5e054fd62c47945d5))

  Support added by bumping dependency on RPPAL.

- Improved API stability expectations. ([fb663f5](https://github.com/solimike/rppal-pifacedigital/commit/fb663f580dcd1d9459f4afff70737d8c4dbf9de5))

  This crate has worked with very few issues
  for many months. V0.1 recognises
  the expectation that there will not be many
  breaking changes in the future.

### Miscellaneous Tasks

- Bump dependency on RPPAL to V0.19. ([509464b](https://github.com/solimike/rppal-pifacedigital/commit/509464bae82873d8125b9c1c9269cebff96262ae))

  Some minor API breakages but none that break our API.

## [0.0.5] - 2023-01-22

[5549d05](https://github.com/solimike/rppal-pifacedigital/commit/5549d051bac5b2b48ce6dfc7e4aac7a7f70ce3d4)...[b02f0ca](https://github.com/solimike/rppal-pifacedigital/commit/b02f0ca114a4fb168c3bf333ff7f8bab7ea4b736)

### Features

- Async interrupts ([#7](https:&#x2F;&#x2F;github.com&#x2F;solimike&#x2F;rppal-pifacedigital&#x2F;issues&#x2F;7)) ([cadfa78](https://github.com/solimike/rppal-pifacedigital/commit/cadfa78bfb03062dd5a520b6b8313d2a160f6f26))

  Support async interrupts: when an interrupt occurs, the specified callback or closure
  gets invoked from its own thread.

## [0.0.4] - 2023-01-20

[c29ceef](https://github.com/solimike/rppal-pifacedigital/commit/c29ceefda262ba88d2546ac18a70a5d4a9a7df4c)...[5549d05](https://github.com/solimike/rppal-pifacedigital/commit/5549d051bac5b2b48ce6dfc7e4aac7a7f70ce3d4)

### Fix

- Fixed the build status badge. ([#6](https:&#x2F;&#x2F;github.com&#x2F;solimike&#x2F;rppal-pifacedigital&#x2F;issues&#x2F;6)) ([b23fdf0](https://github.com/solimike/rppal-pifacedigital/commit/b23fdf0c653f91a0779b1a810613eb6bc48daa02))

  Broken by breaking change.
  See: https:&#x2F;&#x2F;github.com&#x2F;badges&#x2F;shields&#x2F;issues&#x2F;8671

## [0.0.3] - 2022-11-14

[6a93a4b](https://github.com/solimike/rppal-pifacedigital/commit/6a93a4b96a502f4cd78a7082dad3b2d8a68c19f4)...[c29ceef](https://github.com/solimike/rppal-pifacedigital/commit/c29ceefda262ba88d2546ac18a70a5d4a9a7df4c)

### Miscellaneous Tasks

- Relax dependency on `rppal_mcp23s17` crate. ([314e043](https://github.com/solimike/rppal-pifacedigital/commit/314e043a8665e6b2ca7fb89fc42ccfd6e7fed87a))

## [0.0.2] - 2022-11-14

[4dd2cad](https://github.com/solimike/rppal-pifacedigital/commit/4dd2cadc414509f6186863f77454880a614e29d3)...[6a93a4b](https://github.com/solimike/rppal-pifacedigital/commit/6a93a4b96a502f4cd78a7082dad3b2d8a68c19f4)

### Documentation

- Basic badges. ([#2](https:&#x2F;&#x2F;github.com&#x2F;solimike&#x2F;rppal-pifacedigital&#x2F;issues&#x2F;2)) ([d07a4aa](https://github.com/solimike/rppal-pifacedigital/commit/d07a4aa0c6c4eb9f65002ed474ee4cf038a5d2cc))

  Add some badges to the README so users get some idea of the status on crates.io _etc_. Until some CI is set up, 
  the build badge remains a &quot;TODO&quot;.

### Features

- Add examples ([#4](https:&#x2F;&#x2F;github.com&#x2F;solimike&#x2F;rppal-pifacedigital&#x2F;issues&#x2F;4)) ([7e0da6d](https://github.com/solimike/rppal-pifacedigital/commit/7e0da6d18ec96e786820b43fa7d07ee5bbe50c1a))

  Added some examples in the `examples` folder:
  
  - Blinking LED with interrupt handling.
  - SPI bus speed and reliability test.

### Testing

- Feature flag `mockspi` ([#3](https:&#x2F;&#x2F;github.com&#x2F;solimike&#x2F;rppal-pifacedigital&#x2F;issues&#x2F;3)) ([efd2c31](https://github.com/solimike/rppal-pifacedigital/commit/efd2c319bc2afa0b374d8a1dd520d6ef0b96c9b9))

  The `mockspi` feature enables testing without access to the Raspberry Pi hardware.

- Basic CI workflow ([#5](https:&#x2F;&#x2F;github.com&#x2F;solimike&#x2F;rppal-pifacedigital&#x2F;issues&#x2F;5)) ([665babc](https://github.com/solimike/rppal-pifacedigital/commit/665babc6c1c6415d01de04feea4013afb34ee2e1))

  Run a basic workflow to check the integrity of the repo on every push and PR.

## [0.0.1] - 2022-11-11

### Features

- &quot;MVP&quot; PiFace Digital I&#x2F;O driver ([#1](https:&#x2F;&#x2F;github.com&#x2F;solimike&#x2F;rppal-pifacedigital&#x2F;issues&#x2F;1)) ([9106240](https://github.com/solimike/rppal-pifacedigital/commit/9106240d78d2c28e5ed7be1a00260d73cbef520a))

  This initial release supports:
  
  - Exposing the underlying capabilities of the *rppal-mcp23s17* crate.
  - Augmenting the `InputPin` with synchronous interrupt capabilities.
  - Polling for synchronous interrupts from a single `InputPin`.
  - Polling for synchronous interrupts across many `InputPin`s.

<!-- generated by git-cliff -->
