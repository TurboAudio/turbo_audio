require("libs.framework")
require("libs.colors")

SettingsSchema = {}

local view = 800
local tick = 0

function Tick()
	tick = tick + 1
	for i = 0, #Colors - 1 do
		local step = view / #Colors
		local value = math.min(Fft_Result:get_frequency_amplitude(i * step) * 5, 255)
		local hue = (i + tick) % #Colors / #Colors
		local r, g, b = HsvToRgb(hue, 1, 1)
		Colors[i + 1].r = r / 255 * value
		Colors[i + 1].g = g / 255 * value
		Colors[i + 1].b = b / 255 * value
	end
end
