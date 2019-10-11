pragma solidity ^0.5.0;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";

contract Token is ERC20 {
    constructor() public {
        _mint(msg.sender, 1000000000000000000000000000);
    }
}
