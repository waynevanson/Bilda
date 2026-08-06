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
10. Evaluation is kinda lazy by default. Futures are easier to manage via thunks.

The language could be all about processes?

I mean it is a runtime, right?

A scope thing to cache a thing?

Everything is a function? lol

```
vars
  Thing = ~ String
  Thang = String

  Name1 = + {
    First = Thing
    Second = Thang
  }

  Name2 = * {
    First = Thing
    Second = Thang
  }

  Name3 = * {
    Name1
    Name2
  }

  Name4 = + {
    Name1
    Name2
  }

  Name5 = + {
    0 = Name1
    1 = Name1
  }

  Name6 = * {}

  Try {
    Name = String,
    Age = U32
  } = {
    Ok = {}
  }

  Name3 = - {
    First = {},
    Second = {},
  }

  # type access
  f_oath = Name4::Name1

  f_cunt = f_oath::Name1

  # value access
  f_vark = Name3.First

  closure = penis brains => penis * brains
  clos = penis => closure 3
  ure = clos 5
  closed = closure penis brains
expr
  name3

# Comment
# Types are easy but what about real expressions?

partitioned list = partition list {}
partitioned = fmap partitioned (sd => sd)

```

## Language Considerations

It's hard to get everything a language needs whilst also keeping verbosity low.

I believe it's better to do things 1 way rather than have an optimal way to do the same thing in 2 different cases.

### Features and things softsware at code level could do

1. Variables
2. Expressions
3. Assignment
4. Functions
5. Lambdas
6. Named returns
7. Strategy pattern (traits)
8. Modules
   1. Imports
      1. Named
      2. Default
   2. Exports
      1. Named
      2. Defaults
9. Public
10. Private
11. Scope of variables
12. Reference, Dereferences
13. Compiled, Interpreted
14. Overridable symbols
15. Total or Turing complete?

If the language doesn't have good built-ins, the ecosystem is forced to fill the gap.
