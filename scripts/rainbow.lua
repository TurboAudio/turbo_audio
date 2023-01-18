require("scripts.libs.framework")
require("scripts.libs.colors")

SettingsSchema = {}

local tick = 0;

function Tick()
	for index = 0, #Colors - 1 do
		local hue = (index + tick) % #Colors / #Colors
		local r, g, b = HsvToRgb(hue, 1, 1)
		Colors[index + 1].r = r
		Colors[index + 1].g = g
		Colors[index + 1].b = b
	end

    tick = tick + 1
end
