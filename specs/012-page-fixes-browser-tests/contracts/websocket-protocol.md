# Contract: `hello.band_groups` (protocol 4, narrowed)

The shape is unchanged from feature 011:

```json
"band_groups": [ { "name": "hf-contest", "label": "HF contest", "bands": ["160m", "80m", "40m", "20m", "15m", "10m"] } ]
```

**What changes:** when two or more groups would list the same `bands`, only the most specific
one is sent. That is the one with the fewest bands in its full definition, and on a tie the first
in the group order. The order of the remaining groups is unchanged.

| Server band setting | `band_groups` names |
|---------------------|---------------------|
| none (default) | `hf-contest` |
| `hf` | `hf`, `hf-contest`, `warc` |
| `all` | `lf-mf`, `hf`, `hf-contest`, `warc`, `vhf`, `uhf`, `shf`, `ehf` |

`PROTOCOL_VERSION` stays 4: no field changes, and a page that handled a longer list handles a
shorter one.
