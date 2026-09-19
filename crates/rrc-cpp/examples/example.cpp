#include "rrc.h"
#include <iostream>

int main() {
    try {
        std::cout << "Radial Response Code (RRC) C++ API Demo\n";
        auto info = rrc::get_version_info(1);
        std::cout << "RRC V1: " << info.rings << " rings, " 
                  << info.bits << " bits, "
                  << info.capacity_bytes_m << " bytes capacity.\n";

        std::string payload = "https://github.com/MdSagorMunshi/rrc";
        std::string svg = rrc::encode_svg(payload, rrc::EccLevel::M);
        std::cout << "Generated SVG (" << svg.length() << " characters).\n";

        auto png = rrc::encode_png(std::vector<uint8_t>(payload.begin(), payload.end()));
        std::cout << "Generated PNG (" << png.size() << " bytes).\n";
    } catch (const rrc::Error& e) {
        std::cerr << "RRC Error [" << e.code << "]: " << e.what() << "\n";
        return 1;
    }
    return 0;
}
