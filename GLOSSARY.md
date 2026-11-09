# Ball Simulator

Generates short, satisfying 2D videos of balls bouncing inside an arena, where every boundary hit makes a sound and may trigger an effect.

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

**Hook Title**:
The short premise shown above the Arena and used as the Video's title (e.g. "Every bounce it doubles").

**Heat**:
How many balls overlap at a point in the Arena, shown as colour that reaches white-hot at the Population Cap.

**Counter**:
The live number shown below the Arena (ball count, Boundary Hit count, or speed), chosen by the Scenario.

**Quality Gate**:
A headless dry run of a Seed that rejects it if the resulting Video would be dull (ends too soon, never ends, too few Boundary Hits).

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

**Boundary Hit**:
A ball touching the Boundary; the default trigger for sounds and on-hit Rules. Balls pass through each other unless a Rule says otherwise.
_Avoid_: Collision, bounce, edge hit

### Audio

**Voice**:
One currently sounding note; the number of simultaneous Voices is capped to keep audio clean.
