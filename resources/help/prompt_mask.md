# Prompt Mask

This module offers methods to mask (decorate) data that has
been typed on the prompt line but not yet sent to the server.

See also `/help prompt`.

##

***prompt_mask.set(data, table) -> bool***
Set the prompt mask table to be associated with the input data. Returns true
if the mask is valid, and the prompt data hasn't changed. Returns false if
the mask is not valid, or the prompt data has changed and no longer matches
the data argument.

Multiple calls to `prompt_mask.set` will merge the tables, with colliding 
keys having their value replaced by the value from the last call.

- `data`    Prompt data to mask. Must match current prompt input data.
- `table`   A Lua table of integer index keys and mask content values.
            Each index must be a valid character index within the bounds
            of data (1-indexed) and must fall at a character boundary.


```lua
-- Example: Highlight any lines that start with a dangerous command.
prompt.add_prompt_listener(function()
    local danger = "/quit"
    local data = prompt.get()
    if #data < #danger or string.sub(data, 1, #danger) ~= danger then
        return
    end
    local danger_mask = {[1] = BG_RED, [#data+1] = C_RESET};
    local res = prompt_mask.set(data, danger_mask);
    blight.output(string.format("masked: %s",res))
end)
```

##

***prompt_mask.clear()***
Clear the current prompt mask table (if any).

##

***prompt_mask.get() -> table***
Return the current prompt mask table (if any).

- `table`   A Lua table of integer index keys and mask content values
            previously set with calls to `prompt_mask.set`.
            Each index key will be a character index within the bounds
            of the current prompt input data (1-indexed).