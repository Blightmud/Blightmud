# FS

This module allows interacting with the filesystem on your local machine.
For regular file read/write operations please refer to luas standard `io` module.

##

***fs.monitor(path, callback)***
Monitor a directory for file operations.

This will trigger events for any read, write, remove and create operations
inside the provided directory (recursive). Due to various differences between
operating systems there is no agnostic way of knowing what type of event
occured. If something happens with a file you will get an event.

- `path`        The path to monitor
- `callback`    The callback function. Should accept an FSEvent (see below)

The monitor has a standard de-bounce of 5 seconds. This means that repeated
writes to the same file within 5 seconds will only trigger one `write` event.

It also means that Blightmud will be notified of a write 5 seconds after the
first write event.

## The FSEvent object
```lua
FSEvent = {
    paths = ["/a/path/to/the/affected/file", "/another/file"]
}
```

## Example
```lua
fs.monitor("/home/user/muds", function (event)
    for _,p in ipairs(event.paths) do
        print(string.format("File event: %s", p))
    end
end)
```
