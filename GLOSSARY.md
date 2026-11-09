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
The event that ends a Video's action (Population Cap reached, Arena filled, ball escaped), backed by a hard time cap.

**Hook Title**:
The short premise shown above the Arena and used as the Video's title (e.g. "Every bounce it doubles").

**Counter**:
The live number shown below the Arena (ball count, Boundary Hit count, or speed), chosen by the Scenario.

**Quality Gate**:
A headless dry run of a Seed that rejects it if the resulting Video would be dull (ends too soon, never ends, too few Boundary Hits).

**Population Cap**:
The maximum number of balls in a Video; reaching it is the End Condition for splitting Scenarios.

### Simulation

**Arena**:
The shape that contains the balls; its edge is the Boundary.
_Avoid_: World, container, edge

**Boundary**:
The edge of the Arena that balls bounce off.
_Avoid_: Wall, edge

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
