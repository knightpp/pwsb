# pwsb - pipewire sound board

I've never worked with audio before, so it was hard. But with some luck and persistence I managed
to finish this project 😮‍💨.

## How it works

- create an output pipewire node
- if `target` is specified than set `target` property on the node
- depending on presence of the target node it will play sound to the default output device or
to the `target` node (for example discord).

## How the progam is meant to be used

Run the CLI with two arguments: `file` and `target`. File is a path to some supported audio file.
Target is a name of a pipewire node, to get the name use Helvum, it is the label on top of every node.

## Used resources

- [pipewire's examples folder](https://gitlab.freedesktop.org/pipewire/pipewire-rs/-/tree/main/pipewire/examples)
- [helvum](https://gitlab.freedesktop.org/pipewire/helvum) useful to see and rearrange nodes
- [pipewire tutorial](https://docs.pipewire.org/page_tutorial1.html). The tutorial is for C, but
some solutions are available in the examples folder in rust
- [symphonia](https://github.com/pdeljanov/Symphonia/blob/master/GETTING_STARTED.md)
