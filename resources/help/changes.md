# Changes in Blightmud $VERSION

## The GMCP module has been re-worked
All the gmcp related functionality now resides in the `gmcp` module.
It's already imported and ready to use. The following changes to your scripts will let you have a smooth transition.

`blight:on_gmcp_ready` is now referenced as `gmcp.on_ready`
`blight:register_gmcp` is now referenced as `gmcp.register`
`blight:add_gmcp_receiver` is now referenced as `gmcp.receive`
`blight:send_gmcp` is now referenced as `gmcp.send`
