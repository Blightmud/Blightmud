# JSON

JSON encoding and decoding is provided by [](https://github.com/rxi/json.lua).

It provide the following functions

***json.encode(value)***

Returns a string representing a value encoded in json

```lua
json.encode({ 1, 2, 3, { x = 10 } }) -- Returns '[1,2,3,{"x":10}]'
```

***json.decode(value)***

Returns a value representing the decoded json string

```lua
json.decode('[1,2,3,{"x":10}]') -- Returns { 1, 2, 3, { x = 10 } }
```
