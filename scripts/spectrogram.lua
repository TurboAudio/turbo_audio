require("scripts.framework")

SettingsSchema = {
}

local view = 600;
local tick = 0;

function Tick()
    local local_view = view + math.sin(tick) * 200;
	for i = 0, #Colors - 1 do
		local step = local_view / #Colors;
		local value = math.floor(math.min(Fft_Result:get_average_amplitude(i * step, i * step + step) * (i + 1) / 20, 255));
		Colors[i + 1].r = value;
		Colors[i + 1].g = value;
		Colors[i + 1].b = value;
	end
    tick = tick + 1 / 60;
end
