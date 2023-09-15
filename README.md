# Pillars

**DO NOT USE THIS LIBRARY** :)

I don't know enough unsafe rust to know exactly why this library is unsound, but
I'm CONFIDENT there are LOTS of unconsidered aspects to this. I'm mostly using
it to learn and welcome any PRs which solve the spirit of the problem in a sound
way.

The idea of this library is to use a message passing system for tokio
tasks which routes messages amongst tasks based solely on the type of 
message being sent. This allows a task to send a message of type `T` and have
that message routed to running task which is listening for messages of type `T`.
