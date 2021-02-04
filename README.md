# Helltaker Solver

A little pet project which finds an optimal solution for a given puzzle room of Helltaker, a free little video game available on Steam with some very catchy music. The tool uses a memoized search, printing the solution as a sequence of the puzzle's state, laid out over the course of its solution. 

The main interesting things about the implementation are:
1. Dense game state encoding to minimize cost of replicating states in memory
1. Storage of encountered configurations in a map cache, a memoization technique that (a) facilitates the "bound"-half of the branch-and-bound search, and (b) laying the foundation for a paralellized implementation based on transposition-driven scheduling.

Quirks:
1. The input is hardcoded (as one string literal)
1. The board size max is hardcoded (in one single const definition)

```took 56.1068ms
###########
#### G ####
####OLO####
##O#O  # ##
#O  OOO  K#
# OOO  OO #
##@ O  O ##
###########

Right
###########
#### G ####
####OLO####
##O#O  # ##
#O  OOO  K#
# OOO  OO #
## @O  O ##
###########

Up
###########
#### G ####
####OLO####
##O#O  # ##
#O OOOO  K#
# O O  OO #
## @O  O ##
###########

... 
```
