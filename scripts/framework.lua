Colors = {}

function Set_colors()
	local data = ""
	for _, value in pairs(Colors) do
		local rgb_triplet_binary = string.char(value.r) .. string.char(value.g) .. string.char(value.b)
		data = data .. rgb_triplet_binary
	end
	Colors_bin = data
end

function Resize_Colors(len)
	if len ~= #Colors then
		Colors = {}
		for index = 1, len do
			Colors[index] = { r = 0, g = 0, b = 0 }
		end
	end
end
