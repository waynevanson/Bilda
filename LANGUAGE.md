how to call ?

identifier arg arg arg
identifier
identifier (arg, arg, arg)

how to pass around functions as parameters if

nix style rec

```bilda
let
  Status = + {
    completed = True
    duration =  u32
  }
  Chore = * {
    title = String
    description = String
    status = Status
  }
in
Chore {
  title = "Vacuum"
  description = "Get the machine do the sucky in every room"
  status = Status {
    completed = True
  }
}
```
