# FS

This module allows interacting with the filesystem on your local machine.
For regular file read/write operations please refer to luas standard `io` module.

##

***fs.monitor(path, callback)***
Monitor a directory for file operations.
This will trigger events for any read, write, remove and create operations
inside the provided directory (recursive).

- `path`        The path to monitor
- `callback`    The callback function. Should accept an FSEvent (see below)

The monitor has a standard de-bounce of 5 seconds. This means that repeated
writes to the same file within 5 seconds will only trigger one `write` event.

It also means that Blightmud will be notified of a write 5 seconds after the
first write event.

## The FSEvent object
```lua
FSEvent = {
    event = "event",
    paths = ["/a/path/to/the/affected/file", "/another/file"]
}
```

### Event values

- `"write"`   A write event to a file occured
- `"create"`  A file was created
- `"remove"`  A file was removed
- `"rename"`  A file was renamed (two paths provided)
- `"event"`   Unspecified event (should probably be ignored)
- `"undef"`   An undefined event (can safely be ignored)

## Example
```lua
fs.monitor("/home/user/muds", function (event)
    if event.event == "write" then
        print(string.format("File saved: %s", event.paths[1]))
        reload_my_scripts()
    elseif event.event == "rename" then
        print(string.format("File moved: %s -> %s", table.unpack(event.paths)))
        script.load(event.paths[2])
    end
end)
```
