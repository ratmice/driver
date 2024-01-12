###
This is just an experiment.

There may be good ideas in here, it isn't especially readible
The number of traits really might be excessive.

Lets give an overview overview of the builder pattern alternative used here: 

### Driver variation of init-struct-pattern

Driver uses a variation of the [init-struct-pattern](https://xaeroxe.github.io/init-struct-pattern/), an alternative to the builder pattern.
this case, let us quickly review the init-struct pattern, and the variations on it used in builder in a simplified example:


### Normal init-struct-pattern
```
mod foo {
   #[derive(Default)]
   pub InternalDefault;
}

#[derive(Default)]
pub struct FooConfig {
   #[doc(hidden)]
   _non_exhaustive: InternalDefault;
}

impl FooConfig {
   fn init(self) -> Foo {
      /// do the stuff you would normally do in build.
   }
}

let foo = FooConfig::default().init();
```

### We can add some required arguments...

```
#[derive(Default)]
pub struct FooOptionalArgs {
   #[doc(hidden)]
   _non_exhaustive: InternalDefault;
}
/// We no longer derive default here anymore:
pub struct FooConfig {
   name: String,
   optional_args: FooOptionalArgs,
}

/// Impl is the same.

FooConfig{ 
   name: "foo",
   optional_args: Default::default(),
}.init();
```
### Make it generic over *another* init-struct-pattern

```
pub struct FooConfig<X: Bar> {
  name: String,
  foo_options: FooOptionalArgs,
  x_options: (X::RequiredArgs, X::OptionalArgs)
}

impl FooConfig<X: Bar> {
  fn init(self) -> X::Output
  where (X::RequiredArgs, X::OptionalArgs): Into<Bar::Config>
  {
    // Do a bunch of stuff,
    ...
    // then call BarConfig.
    let x: Bar::Config = self.x_options.into();
    x.init()
  }
}

```

### 
Along the way we can build and make some `Driver` specific arguments
`FooStuff` to pass in to `FooConfig::init` alongside `BarConfig`.
Additionally `Foo::init` can return `(FooStuff, Bar::Output)`.

At this point we've reached the general idea behind driver, except
it also has a bunch of program specific trait bounds on it's `Bar` trait
associated types.

This is a basic overview of how it works without getting into too much detail.


