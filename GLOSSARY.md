# Ball Simulator

Generates short, satisfying 2D videos of balls bouncing inside an arena, where every boundary hit makes a sound and may trigger a Rule.

## Language

### Videos

**Video**:
One finished output: an encoded clip plus its metadata (title, description, tags, seed, generator version).
_Avoid_: Clip, render, output

**Seed**:
The single number from which every random choice in a Video is derived; the same seed and generator version always produce the same Video.

**Scenario**:
A curated, named combination of Rules with randomised parameters and an End Condition, giving a Video its premise and hook title (e.g. "Every bounce it doubles").
_Avoid_: Preset, template, mode

**Rule**:
One composable piece of behaviour in a Scenario, such as gravity, splitting on a Boundary Hit, or speeding up on a Boundary Hit.
_Avoid_: Effect, modifier, mechanic

**End Condition**:
The event that ends a Video's action by reaching a Terminal State, backed by a hard time cap that only ever rejects the Seed.

**Terminal State**:
A state the Scenario cannot carry on from the way it was, unmistakable to a viewer without reading a number (e.g. a ball filling the Arena).
_Avoid_: Payoff, threshold, limit

**Meltdown**:
A Terminal State in which the Video appears to break down under its own load once the Population Cap is reached.
_Avoid_: Crash, overload

**Ending**:
What follows the End Condition: an optional Play-out, then the frozen final frame held until the cut.

**Play-out**:
The short stretch after the End Condition fires in which the Terminal State finishes on screen (e.g. an escaping ball leaving the frame) before the freeze.

**Finale**:
What the Video draws over the Play-out to show the Terminal State landing (e.g. a vanished ball's pop, the Meltdown's breakdown). Chosen by the End Condition.

**Hook Title**:
The short premise shown above the Arena and used as the Video's title (e.g. "Every bounce it doubles").

**Heat**:
How many balls overlap at a point in the Arena, shown as colour that reaches white-hot at the Population Cap.

**Counter**:
The live number shown below the Arena (ball count, Boundary Hit count, or speed), chosen by the Scenario.

**Quality Gate**:
A headless dry run of a Seed that rejects it if the resulting Video would be dull (ends too soon, never ends, too few Boundary Hits, stalls hunting the last of the Film, or is won by a ball that fills the Arena).

**Population Cap**:
The maximum number of balls in a Video; splitting stops once it is reached.

### Simulation

**Arena**:
The shape that contains the balls; its edge is the Boundary.
_Avoid_: World, container, edge

**Boundary**:
The edge of the Arena that balls bounce off.
_Avoid_: Wall, edge

**Gap**:
An opening in the Boundary that balls pass through instead of bouncing off.

**Crack**:
A mark a Boundary Hit leaves in the Boundary, longer and more branched the closer the ball was to max speed. Balls still bounce off a cracked Boundary.

**Shatter**:
The Boundary breaking all the way round at the first Boundary Hit at max speed: its shards fly outward and the ball Escapes.

**Escape**:
A ball passing out through an opening a Rule made in the Boundary (such as a Gap). An escaping ball is no longer held by the Boundary or pulled by gravity and drag, so it flies straight out of the frame.

**Film**:
A layer over the inside of the Arena that a ball wipes away wherever it passes; it may grow back from the Film still left. The Arena is cleared when no Film is left.
_Avoid_: Background, coat, fog

**Boundary Hit**:
A ball touching the Boundary; the default trigger for sounds and on-hit Rules. Balls pass through each other unless a Rule says otherwise.
_Avoid_: Collision, bounce, edge hit

### Audio

**Voice**:
One currently sounding note; the number of simultaneous Voices is capped to keep audio clean.

**Scale**:
The set of notes a Video's Boundary Hits can play, chosen by the Seed unless the Scenario pins one.
_Avoid_: Key, mode

**Pitch Mode**:
How a Boundary Hit's note is picked from the Scale: stepping up and back down it, by where the hit lands on the Boundary, or by ball size.
