require("scripts.libs.framework")
require("scripts.libs.colors")

SettingsSchema = {}

local view = 1500
local tick = 0

local values = {}
local tmpColors = {}

function Tick()
	tick = tick + 1
	for i = 0, #Colors - 1 do
		local step = view / #Colors
		local value = math.min(Fft_Result:get_frequency_amplitude(i * step) * 3, 255)
		if values[i + 1] == nil then
			values[i + 1] = value
		else
			values[i + 1] = math.max(value, values[i + 1] * 0.95)
		end

		local hue = (i + tick) % #Colors / #Colors
		local r, g, b = HsvToRgb(hue, 1, 1)
		value = values[i + 1]
		-- tmpColors[i + 1] = { r = r / 255 * value, g = g / 255 * value, b = b / 255 * value }
		Colors[i + 1] = { r = r / 255 * value, g = g / 255 * value, b = b / 255 * value }
	end
	-- for _ = 0, 2 do
	-- 	for j = 1, #Colors - 2 do
	-- 		Colors[j + 1].r = tmpColors[j].r * 0.25 + tmpColors[j + 1].r * 0.5 + tmpColors[j + 2].r * 0.25
	-- 		Colors[j + 1].g = tmpColors[j].g * 0.25 + tmpColors[j + 1].g * 0.5 + tmpColors[j + 2].g * 0.25
	-- 		Colors[j + 1].r = tmpColors[j].b * 0.25 + tmpColors[j + 1].b * 0.5 + tmpColors[j + 2].b * 0.25
	-- 	end
	-- 	Colors[1] = Colors[2]
	-- 	Colors[#Colors] = Colors[#Colors - 1]
	-- 	tmpColors = Colors
	-- end
end
