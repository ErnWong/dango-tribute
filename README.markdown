<div align="center">
  <h1>dango</h1>
</div>

https://user-images.githubusercontent.com/2609018/122154887-04850f00-ceba-11eb-8d83-581ff2405833.mp4

<details>

## What is this?

Just a little experiment made using the [Bevy game engine](https://bevyengine.org/) to get myself a bit more familiar with Rust.

I don't take pride in the quality of this codebase. It is full of hacks, non-idiomatic Bevy code, and more inefficient hacks. Be warned when looking through the code.

The "dango" in question is a tribute to the [Dango Daikazoku](https://www.youtube.com/watch?v=XXDxZ0YGWG8) from Clannad.

## Will I continue working on this?

No. I don't want to ruin the image of Clannad any further. Let's keep this as a small experiment.

## Blob physics

The dangos are octagon meshes, simulated as [FEM surfaces](https://www.nphysics.org/rustdoc/nphysics2d/object/struct.FEMSurface.html) within [nphysics](https://www.nphysics.org/).

To get variable jump heights, it applies a jump force that:
1. spikes when the jump button is pressed, and
2. decays until the jump button is released.
3. drops to zero after the jump button is released.

By default, these FEM surfaces like to roll around. To fix their angle so they're always upright, it measures the aggregate angle and angular momentum of the whole dango using its 8 individual FEM surface elements, and then use something that resembles a [PD control loop](https://en.wikipedia.org/wiki/PID_controller) to lock it in place naturally. To re-enable rolling, we just turn off this PD control loop.

To get the wobbly walking animation, it modulates the horizontal walking force with some squishing and stretching.

Currently, the dango physics don't obey Newton's third law (i.e. the jump force is asymmetrically applied), so if two dangos ever get tangled, they can fly into the air ignoring gravity.

## Blob drawing

To draw the dangos, I used a Catmull-Rom spline from the [Splines crate](https://github.com/phaazon/splines) to smooth the octagon out into 32 vertices, and then used the [lyon crate](https://github.com/nical/lyon) to tessellate them into triangular meshes that can be rendered.

The shadows are just hard-coded octagons on the ground. The eyes are also tessellated using lyon, and are made to blink once in a while using a simple finite state machine.

## Background and post-processing

The background and the hand-drawn look are done in a [post-processing shader](client/assets/shaders/frameshader.webgl2.frag) with the help of a [bevy plugin that's propably not good enough to be made into its own public crate](crates/bevy_prototype_frameshader/src/lib.rs). The fragment shader script is pretty slow and unnecessarily intensive, so I had to limit the size of the screen to keep the framerate consistently smooth. The shader script itself was inspired from [a shadertoy script](https://www.shadertoy.com/view/MsKfRw) by [flockaroo](https://www.flockaroo.at/).

## Music

Yeah, sorry, the short background music was whipped up in a day or two, so it sounds kinda bad. I used [Dorico](https://en.wikipedia.org/wiki/Dorico) to write it up, and then performed it with [Spitfire Audio's BBC Symphony Orchestra Discover's](https://www.spitfireaudio.com/shop/a-z/bbc-symphony-orchestra-discover/) celesta instrument, recorded inside [Cakewalk](http://www.cakewalk.com/).

## Multiplayer

The prediction/rollback logic was eventually refactored out into its own crate: [CrystalOrb](https://github.com/ErnWong/CrystalOrb). This networking code probably isn't efficient, and there are still some bugs that show up when the browser tab sleeps for too long.

## Patches

To get this experiment somewhat working, I hacked together some patches for some dependencies. Some of the patches are genuine fixes and enhancements, but most of them are short-term solutions that are specific to this experiment. When I get time, I'll see if I can make proper pull-requests for some of the fixes back into the upstream repositories.

</details>
