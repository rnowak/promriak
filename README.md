# Promriak

Riak metrics re-exporter in the Prometheus text exposition format.

Note that while it is often *verboten*, `promriak` caches the retrieved metrics
from each Riak instance inbetween scrapes and serves the cached values up until
a stale threshold is reached.

## Configuration

Configuration is first searched for according to
[XDG Base Directory Specification](https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html)
for configuration files.

As a fallback, the following locations will be attempted:

    ./promriak.yaml 
    /usr/local/etc/promriak/promriak.yaml
    /etc/promriak/promriak.yaml

Alternatively, a configuration file path can be provided with the `--config`
command line option or the `PROMRIAK_CONFIG` environment variable, where the
former takes precedence:

    promriak --config /path/to/my_promriak_config.yaml
    PROMRIAK_CONFIG=/path/to/my_promriak_config.yaml promriak

### Configuration Defaults

```yaml
tracing_level: INFO
bind_address: 127.0.0.1
listener_port: 9198
scrape_interval: 2500
stale_threshold: 20000
special_metrics: true
prefix: riak_
instances:
  - id: local
    endpoint: http://127.0.0.1:8098/stats
```

### Configuration File

Please see [misc/promriak.example.yaml](misc/promriak.example.yaml) for an
annotated example configuration file.

YAML is a superset of JSON. Do as you wish with that information.

## License

Dual-licensed under:

- Apache License, Version 2.0
- MIT License
