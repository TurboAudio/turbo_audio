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

function Tick()
    for _ = 0, 1 do
        for index = 0, #Colors - 2 do
            Colors[#Colors - index].r = Colors[#Colors - index - 1].r
            Colors[#Colors - index].g = Colors[#Colors - index - 1].g
            Colors[#Colors - index].b = Colors[#Colors - index - 1].b
        end
    end
	Colors[1].r = math.floor(math.min(Low_Frequency_Amplitude, 255))
	Colors[1].g = math.floor(math.min(Mid_Frequency_Amplitude, 255))
	Colors[1].b = math.floor(math.min(High_Frequency_Amplitude, 255))
end
