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
	local new_r = math.floor(math.min(4 * Fft_Result:get_frequency_interval_average(0, 100), 255))
	local new_g = math.floor(math.min(15 * Fft_Result:get_frequency_interval_average(100, 1000), 255))
	local new_b = math.floor(math.min(20 * Fft_Result:get_frequency_interval_average(1000, 2000), 255))
	  for _ = 0, 1 do
	      for index = 0, #Colors - 2 do
	          Colors[#Colors - index].r = Colors[#Colors - index - 1].r
	          Colors[#Colors - index].g = Colors[#Colors - index - 1].g
	          Colors[#Colors - index].b = Colors[#Colors - index - 1].b
	      end
	Colors[1].r = new_r
	Colors[1].g = new_g
	Colors[1].b = new_b
	  end
end
