Build systems are interesting and have a bunch of properties, and require solutions for the problems they face.
I think the flexibility is one of the hardest parts to manage.

What would a language need?

1. Dependency graph
2. Lazily evaluated? I don't think it's required.
3. Turing completed? Not really sure what it means.
4. Processes as a keyword concept: series, parallel, concurrency
5. Sum/Product types. Should these be designed when adding platforms and stuff that explodes like features?
6. Remote cache, execution and sharding.
7. Plugin system.
8. Exec on different platforms?
9. Dynamically generate inputs and outputs?

The language could be all about processes?

I mean it is a runtime, right?

A scope thing to cache a thing?

```
name1 = sum {
  First = Thing,
  Second = Thang,
}

name2 = product {
  First = Thing,
  Second = Thang,
}

match name1 {
  First => {},
  Second => {},
}
```
