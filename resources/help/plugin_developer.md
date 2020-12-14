# Developing plugins

What follows are a few pointers if you intend to share your script as a plugin.

## Structure
Your plugin must have a `main.lua` file in the root of the project. From this
file you may `script.load` or `require` whatever your plugin uses. But it all
has to start in the `main.lua` file.

Beyond the `main.lua` requirement you may create folders and files as you see
fit.

## Aliases, Triggers etc.
Your plugin can create anything a regular blightmud script can create.
Everything it does create will be available and seen by the user (eg. Through
`/aliases` or `/triggers`). This might change in the future but for now that's
how it works.

## Hosting/Sharing
Make your plugin available in a git repository. There are numerous options for
this so I don't think it needs further explanation.

Blightmud will always update (git pull) from the `master` branch. So making
this is your 'stable' branch and handling development on another branch is a
good tip.
