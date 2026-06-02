# MDD Philosophy

MDD (Markdown with Diagrams) is a lightweight Markdown preprocessor.

MDD scans Markdown code blocks, invokes external plugins, and replaces the blocks with generated SVG images.

Plugins are simple executable commands discovered from `$PATH`.

For a code block:

````markdown
```sequence
Alice -> Bob: Hello
```
````

MDD executes:

```bash
mdd-sequence
```

The block content is passed through stdin, and the plugin returns SVG through stdout.

```text
Markdown
    ↓
Code Block
    ↓
mdd-{block-name}
    ↓
SVG
    ↓
Markdown
```

MDD itself only handles:

* Markdown parsing
* Plugin discovery
* Plugin execution
* Markdown generation

Diagram rendering logic belongs entirely to plugins.
