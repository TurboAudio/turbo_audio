require("scripts.libs.framework")

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
local tickCount = 0

function Tick()
	for index = 1, #Colors do
		Colors[index].r = (tickCount + index) % 256
	end
	tickCount = (tickCount + 1) % 256
end
