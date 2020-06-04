# Terracing
Using 2D simplex noise to generate inked-style topographical maps (I'm thinking maps and levels for some video game).
Important self-imposed requirement: each pixel is 100% independently sampled.

Generation of one cell involves distinct steps:
1. determine height from a set of 1st order and 2nd order noise fields (2nd order: field is sampled from output of another sample)
1. terracing. heights are clamped at regular intervals
1. regions of terraced land are pushed up and down to break up contiguous regions of terrain.
1. terrace boundaries are turned into cliffs (Drawn dark), except for some regions ("ramps").
1. a water table is generated from a noise field, and submerged regions are drawn underwater.

![Example Image](https://github.com/sirkibsirkib/terracing/tree/master/example.png)