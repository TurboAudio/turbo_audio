require("scripts.framework")

SettingsSchema = {
	title = "TurboSettings",
	type = "object",
	required = {
		"enable_beep_boops",
		"intensity",
	},
	properties = {
		enable_beep_boops = {
			type = "boolean",
		},
		intensity = {
			type = "integer",
			format = "int32",
			maximum = 10.0,
			minimum = 0.0,
		},
	},
}

-- Local state
local tick_count = 0

function Tick()
	for index = 1, #Colors do
		Colors[index].r = (tick_count + index) % 256
	end
	tick_count = (tick_count + 1) % 256
end
