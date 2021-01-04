# Motivation

When writing games, it's common to want to find an entity that has a component with a particular value. 
In one particularly important case, suppose we want to find all entities in a particular `Position` on our grid.

In Bevy, this is straightforward enough: query for that component, iterate through it, then see which values match.
But doing so incurs a pretty heavy cost: every time we need to do this, we're scanning through every single entity with that component, and this operation takes linear time.

In a game where this is a common operation, and the number of entities to search is large, it would be nice to be able to have a quick way to look this up.
Create some sort of hash map from your component's value to their entity, insert your components when they're changed, and then look it up in nice constant time.

Sounds easy right? But getting all the details right is surprisingly devilish, performance is at a premium and bugs deep in your internal code can be a nightmare to debug.

Hence: a proposal for an Official Bevy Index.

# Constraints
1. Must be faster than simply iterating through entities for reasonably large collections (at least n=10000, ideally n=1000).
2. The overhead of creating and maintaining the index shouldn't outweigh the benefits gained, especially for slowly-changing components.
3. Using these indexes should be ergonomic: low-boilerplate, no obvious pitfalls, no need for manual updating.
4. The values provided must always reflect the underlying game state. 
5. When multiple entities have a component with the same value, choose whether to extract all of those entities, or the first one.
6. As shown below, any system that uses an `Index<T>` needs the same scheduling constraints as one that reads `T` directly. 

# Approaches

In all cases, our goal is to provide our system a set of all entities for which a specified component has the given value.

## Iteration
As a baseline, here's how you might do this with iteration.

```rust
fn my_system(query: Query<&MyComponent>) {
	let mut set = HashSet::new();
	for c in query.iter().cloned(){
		if *c == target {
			set.insert(*c);
		}
	}
}
```

Clean and easy, but the linear time will be a real deal-breaker for some applications.

## Naive Hashmap

So, let's use a hashmap!
We want to look up our entities by their component value, so we'll go with a `Hashmap<MyComponent, Entity>`.
Stick in your target value, and get out an entity.

But wait: what if we have multiple entities with the same value? 
We'll only get one of them back out, and updating our data structure will overwrite other good values.

## Simple Multimap

Alright, so we'll swap to a multimap: so we can store multiple values for each given key.
Problem solved, right?

Now we just need to add some nice updating logic and we can get back to actually making our game.
So what do we need to do?

We don't want to completely rewrite our index every time we update it (otherwise why aren't you just using iteration?), so we need to modify only the values that we've found.
We can grab a list of components that will need to be updated with Bevy's handy `Changed` functionality.
We'll need to be particularly careful here to remove the values for entities that have been removed: otherwise we'll end up calling `.get` on invalid entities and throwing panics.

But then how do we know where the old values that we need to remove are?

## Once-per-frame Multimap-Powered Index

We could iterate through the dictionary in order, but now we're back to linear (in the number of unique component values) time.
Instead, we need a reverse mapping too. 
We can't really on Bevy's built-in queries with `.get` here, since we need the old value of our component.
So instead, we're left with creating and maintaining a reverse mapping on our own, which can be a simple hashmap since we know every entity will have exactly one component.

Wrap both the forward and reverse mappings in a custom `Index` struct and we can move on to scheduling our updates!
Update it after startup, and then run it at the end of each frame, spin up a nice little utility system to initialize all this and ta-da, effortless indexing right?

Not so much: the values of these components can readily change within the frame, resulting in us working off of stale values to bizarre and catastrophic effect.

## Manually-timed Multimap-Powered Index

Alright, so we need to make sure that our index is fresh every time that we're using it.
There are two broad approaches here:

1. Update the index after each system that changes the underlying component.
2. Update the index before each system that uses the index.

If a single value of our component is changed multiple times before we use it though, we're going to be wasting effort, meaning that the second option is preferred.

So then toss an `update_index<T>.system()` before each time we want to call our index, ensuring that it's fresh.
The boilerplate isn't great but surely this should *always work* (until we forget to add the boilerplate), right?

Unfortunately, the magic of parallelism can work in unexpected ways.
Suppose our system that uses the resource doesn't actually care about our `IndexComponent` directly: it might just be trying to find all units with a given position, rather than modifying position directly.
But that means that it's not querying for `IndexComponent`, merely accessing `Res<Index<IndexComponent>>` and so isn't getting a hard or soft lock on the values from our scheduler in the same way as if it was reading or writing the component through a query.

This means that we can have another system running in parallel, modifying the values of our `IndexComponent` out from underneath us as we're relying on them in a way that could never happen with the iterative approach!
Like with most race-condition-flavored problems, tracking down a bug caused by this behavior sounds particularly nasty.

As an extra wart, we still need to keep the end-of-tick updates too, otherwise we might drop changes, since `Changed` is cleared at the end of each tick.
Unfortunately, this will duplicate all of the work that we've done previously in the current tick.
Keeping around a list of entities that we've updated this tick won't solve this duplication: we can't be sure that the value hasn't changed *again* on us in the mean time.

## Index as a Bevy Primitive

I hope that this design discussion has convinced you, dear reader, that this index functionality is natural, appealing and *deceptively* tricky.
If we're going to be mucking around with engine internals anyways, let's make a nice API for our end users.

1. You can use an index for the component type `<T>` in your systems by adding an `Index<T>` to your function arguments.
   We don't want to use a `Res<Index<T>>` here, because manually mucking around with your indexes is a recipe for disaster and it results in the race-condition bug described above.
2. An index for the type `T` should be automatically initialized and maintained (like `Local`) whenever we have at least one system is added that has an argument of `Index<T>`.
   Doing this manually is boilerplate, but also easy to screw up if you add it after values of the component start changing.
3. Indexes have one public method `.get(k: T) -> HashSet<Entity>` (which may be empty).
The order that entities are stored in our index is completely arbitrary, so we don't want to expose it lest people try an rely on it. 
4. All of the updating should happen magically under the hood, and should *Just Work* no matter what weird systems the user might dream up.

This lets us write our initial example code as: 

```rust
fn my_system(index: Index<MyComponent>) {
	let set = index.get(target);
}
```

Of course, if n is small or entities change often, performance may be better with simple iteration.

Steps 1, 2 and 3 of this design are easy enough, if we're privileged with access to Bevy's internals.
The real challenge is designing an efficient, foolproof approach to updating. 
Here are some ideas:
1. **Dedicated update systems:** Automatically run an updating system before every system with an `Index` parameter. This is much less painful without the boilerplate, and won't lead to race conditions now that we can tweak our scheduler to be smarter. We'll still need to run the end-of-frame update system though.
2. **Metaprogramming system modification:** Modify the code of every system with an index argument to actually expand out to a `Query` on the appropriate component and combine it into the same step. This lets us avoid modifying the scheduler logic, but it's very hacky and we still need the end-of-tick update system.
3. **DerefMut magic:** Whenever `DerefMut` is called on a component, also update our index if it exists. This wonderful bit of magic is already used by our `Changed` component flag. This works flawlessly, but could have some painful cache issues as we're only inserting one change at a time, and could result in us wasting effort if a component is changed multiple times before our index is used. 
4. **ToIndex component flag:** Whenever `DerefMut` is called on a component, set its `ToIndex` component flag (analogous to `Changed`) to `true`. Automatically schedule a system like in 1, then process all of the updates at once in a batch. Unlike `Changed`, don't reset these flags at the end of the tick. This may result in better branch-prediction, as the work done in our tight component-changing loops is always the same.

I suspect that option 3 or 4 is going to be correct, as they're elegant, foolproof and avoid the end-of-tick cleanup work.
Which one is better will likely come down to benchmarking under realistic-ish workloads.

# Steps to Implement

1. Rewrite the `IntoSystem` macro to accommodate arguments of type `Index`.
2. Ensure that the scheduler treats systems with an `Index<T>` argument as reading `T`.
3. When a system with an `Index` argument is run, hand it the appropriate `Index` struct.
4. Write a nice little struct for holding our forward and reverse maps. See [here](https://github.com/alice-i-cecile/bevy_index/blob/main/src/lib.rs) for an initial attempt.
5. Automatically update the index at the appropriate time, as discussed directly above this section.
6. Write some tests that cover the edge cases listed in this design doc to ensure that it actually works.