# Plugin

Blightmud has a rudimentary way of handling plugins to allow for easier sharing
of lua scripts between users. It's all based off of git.

The following macros exist to help manually adding and loading plugins.

- `/plugins`                    List installed plugins
- `/add_plugin <url|path>`      Install a plugin through a git url or file path
- `/remove_plugin <name>`       Uninstall a plugin
- `/update_plugin <name>`       Update a plugin
- `/load_plugin <name>`         Load a plugin
- `/update_plugins`             Update all installed plugins
- `/enable_plugin <name>`       Toggle a plugin on (autoload)
- `/disable_plugin <name>`      Toggle a plugin off (no autoload)

Plugins are stored in `$DATADIR/plugins`

If you are developing a plugin see `/help plugin_developer`

The following methods exist on the `plugin` module for easy automation and
scripting.

##

***plugin.add(url_or_path)***
Fetches a plugin to your local machine.

- `url_or_path`     The path or url to install the plugin from

##

***plugin.load(name)***
Load a plugins main script into blightmud

- `name`    The name of the plugin

##

***plugin.remove(name)***
Remove a plugin from your local machine

- `name`    The name of the plugin

##

***plugin.get_all() -> {}***
Returns a list of all installed plugins

##

***plugin.update(name)***
Updates a plugin

- `name`    The name of the plugin

##

***plugin.enable(name)***
Toggle plugin autoload on (default after install)

- `name`    The name of the plugin

##

***plugin.disable(name)***
Toggle plugin autoload off

- `name`    The name of the plugin

##

***plugin.enabled() -> {}***
Returns a list of all enabled (autoloaded) plugins
