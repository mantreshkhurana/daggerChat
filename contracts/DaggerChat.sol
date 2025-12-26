// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/**
 * @title DaggerChat
 * @notice Decentralized chat storage with end-to-end encryption support
 * @dev Messages are stored encrypted; only metadata is readable on-chain
 */
contract DaggerChat {
    // ============ Errors ============

    error NotGroupMember();
    error NotGroupAdmin();
    error GroupNotFound();
    error InvalidRecipient();
    error MessageTooLarge();
    error BatchTooLarge();
    error AlreadyRegistered();
    error UserNotRegistered();

    // ============ Constants ============

    uint256 public constant MAX_MESSAGE_SIZE = 4096;
    uint256 public constant MAX_BATCH_SIZE = 50;

    // ============ Structs ============

    struct EncryptedMessage {
        bytes32 id;
        address sender;
        uint64 timestamp;
        bytes encryptedContent;
        bytes32 conversationId;
    }

    struct Group {
        bytes32 id;
        address admin;
        bytes encryptedName;
        bytes encryptedSymKey;
        address[] members;
        uint64 createdAt;
        bool exists;
    }

    struct UserProfile {
        bytes32 publicKeyX25519;
        bytes encryptedMetadata;
        uint64 registeredAt;
        bool exists;
    }

    // ============ State Variables ============

    mapping(address => UserProfile) public users;
    mapping(bytes32 => Group) public groups;
    mapping(bytes32 => EncryptedMessage[]) private conversationMessages;
    mapping(bytes32 => mapping(address => bool)) private groupMembership;
    mapping(bytes32 => uint256) public messageCount;

    // ============ Events ============

    event UserRegistered(
        address indexed user,
        bytes32 publicKeyX25519,
        uint64 timestamp
    );

    event MessageSent(
        bytes32 indexed conversationId,
        bytes32 indexed messageId,
        address indexed sender,
        uint64 timestamp
    );

    event GroupCreated(
        bytes32 indexed groupId,
        address indexed admin,
        uint64 timestamp
    );

    event GroupMemberAdded(
        bytes32 indexed groupId,
        address indexed member,
        uint64 timestamp
    );

    event GroupMemberRemoved(
        bytes32 indexed groupId,
        address indexed member,
        uint64 timestamp
    );

    // ============ User Functions ============

    /**
     * @notice Register a new user with their X25519 public key
     * @param publicKeyX25519 The user's X25519 public key for key exchange
     * @param encryptedMetadata Optional encrypted profile metadata
     */
    function registerUser(
        bytes32 publicKeyX25519,
        bytes calldata encryptedMetadata
    ) external {
        if (users[msg.sender].exists) revert AlreadyRegistered();

        users[msg.sender] = UserProfile({
            publicKeyX25519: publicKeyX25519,
            encryptedMetadata: encryptedMetadata,
            registeredAt: uint64(block.timestamp),
            exists: true
        });

        emit UserRegistered(msg.sender, publicKeyX25519, uint64(block.timestamp));
    }

    /**
     * @notice Update user's public key (for key rotation)
     * @param newPublicKeyX25519 The new X25519 public key
     */
    function updatePublicKey(bytes32 newPublicKeyX25519) external {
        if (!users[msg.sender].exists) revert UserNotRegistered();

        users[msg.sender].publicKeyX25519 = newPublicKeyX25519;

        emit UserRegistered(msg.sender, newPublicKeyX25519, uint64(block.timestamp));
    }

    // ============ Message Functions ============

    /**
     * @notice Send an encrypted message to a conversation
     * @param conversationId The DM or group conversation ID
     * @param encryptedContent The encrypted message content
     * @return messageId The unique message identifier
     */
    function sendMessage(
        bytes32 conversationId,
        bytes calldata encryptedContent
    ) external returns (bytes32 messageId) {
        if (encryptedContent.length > MAX_MESSAGE_SIZE) {
            revert MessageTooLarge();
        }

        messageId = keccak256(
            abi.encodePacked(
                msg.sender,
                conversationId,
                block.timestamp,
                encryptedContent
            )
        );

        EncryptedMessage memory message = EncryptedMessage({
            id: messageId,
            sender: msg.sender,
            timestamp: uint64(block.timestamp),
            encryptedContent: encryptedContent,
            conversationId: conversationId
        });

        conversationMessages[conversationId].push(message);
        unchecked {
            messageCount[conversationId]++;
        }

        emit MessageSent(
            conversationId,
            messageId,
            msg.sender,
            uint64(block.timestamp)
        );

        return messageId;
    }

    /**
     * @notice Send multiple messages in a single transaction (gas optimization)
     * @param conversationIds Array of conversation IDs
     * @param encryptedContents Array of encrypted contents
     */
    function sendMessageBatch(
        bytes32[] calldata conversationIds,
        bytes[] calldata encryptedContents
    ) external {
        uint256 length = conversationIds.length;
        if (length > MAX_BATCH_SIZE) revert BatchTooLarge();
        if (length != encryptedContents.length) revert BatchTooLarge();

        for (uint256 i = 0; i < length;) {
            bytes32 conversationId = conversationIds[i];
            bytes calldata content = encryptedContents[i];

            if (content.length > MAX_MESSAGE_SIZE) {
                revert MessageTooLarge();
            }

            bytes32 messageId = keccak256(
                abi.encodePacked(msg.sender, conversationId, block.timestamp, content, i)
            );

            conversationMessages[conversationId].push(
                EncryptedMessage({
                    id: messageId,
                    sender: msg.sender,
                    timestamp: uint64(block.timestamp),
                    encryptedContent: content,
                    conversationId: conversationId
                })
            );

            unchecked {
                messageCount[conversationId]++;
            }

            emit MessageSent(
                conversationId,
                messageId,
                msg.sender,
                uint64(block.timestamp)
            );

            unchecked {
                ++i;
            }
        }
    }

    // ============ Group Functions ============

    /**
     * @notice Create a new group chat
     * @param encryptedName Encrypted group name
     * @param encryptedSymKey Encrypted symmetric key for the group
     * @param initialMembers Array of initial member addresses
     */
    function createGroup(
        bytes calldata encryptedName,
        bytes calldata encryptedSymKey,
        address[] calldata initialMembers
    ) external returns (bytes32 groupId) {
        groupId = keccak256(
            abi.encodePacked(msg.sender, block.timestamp, encryptedName)
        );

        address[] memory members = new address[](initialMembers.length + 1);
        members[0] = msg.sender;
        for (uint256 i = 0; i < initialMembers.length;) {
            members[i + 1] = initialMembers[i];
            unchecked {
                ++i;
            }
        }

        groups[groupId] = Group({
            id: groupId,
            admin: msg.sender,
            encryptedName: encryptedName,
            encryptedSymKey: encryptedSymKey,
            members: members,
            createdAt: uint64(block.timestamp),
            exists: true
        });

        // Set admin as member
        groupMembership[groupId][msg.sender] = true;

        // Set initial members
        for (uint256 i = 0; i < initialMembers.length;) {
            groupMembership[groupId][initialMembers[i]] = true;
            emit GroupMemberAdded(groupId, initialMembers[i], uint64(block.timestamp));
            unchecked {
                ++i;
            }
        }

        emit GroupCreated(groupId, msg.sender, uint64(block.timestamp));

        return groupId;
    }

    /**
     * @notice Add a member to a group
     * @param groupId The group identifier
     * @param member The address to add
     * @param encryptedSymKeyForMember Symmetric key encrypted for the new member
     */
    function addGroupMember(
        bytes32 groupId,
        address member,
        bytes calldata encryptedSymKeyForMember
    ) external {
        Group storage group = groups[groupId];
        if (!group.exists) revert GroupNotFound();
        if (group.admin != msg.sender) revert NotGroupAdmin();

        groupMembership[groupId][member] = true;
        group.members.push(member);

        emit GroupMemberAdded(groupId, member, uint64(block.timestamp));
    }

    /**
     * @notice Remove a member from a group
     * @param groupId The group identifier
     * @param member The address to remove
     */
    function removeGroupMember(bytes32 groupId, address member) external {
        Group storage group = groups[groupId];
        if (!group.exists) revert GroupNotFound();
        if (group.admin != msg.sender) revert NotGroupAdmin();

        groupMembership[groupId][member] = false;

        emit GroupMemberRemoved(groupId, member, uint64(block.timestamp));
    }

    // ============ View Functions ============

    /**
     * @notice Get messages from a conversation with pagination
     * @param conversationId The conversation to query
     * @param offset Starting index
     * @param limit Maximum messages to return
     */
    function getMessages(
        bytes32 conversationId,
        uint256 offset,
        uint256 limit
    ) external view returns (EncryptedMessage[] memory) {
        EncryptedMessage[] storage messages = conversationMessages[conversationId];
        uint256 total = messages.length;

        if (offset >= total) {
            return new EncryptedMessage[](0);
        }

        uint256 end = offset + limit;
        if (end > total) {
            end = total;
        }

        uint256 resultLength = end - offset;
        EncryptedMessage[] memory result = new EncryptedMessage[](resultLength);

        for (uint256 i = 0; i < resultLength;) {
            result[i] = messages[offset + i];
            unchecked {
                ++i;
            }
        }

        return result;
    }

    /**
     * @notice Check if a user is registered
     */
    function isUserRegistered(address user) external view returns (bool) {
        return users[user].exists;
    }

    /**
     * @notice Get a user's public key for encryption
     */
    function getUserPublicKey(address user) external view returns (bytes32) {
        if (!users[user].exists) revert UserNotRegistered();
        return users[user].publicKeyX25519;
    }

    /**
     * @notice Check group membership
     */
    function isGroupMember(bytes32 groupId, address user) external view returns (bool) {
        return groupMembership[groupId][user];
    }

    /**
     * @notice Get group members
     */
    function getGroupMembers(bytes32 groupId) external view returns (address[] memory) {
        if (!groups[groupId].exists) revert GroupNotFound();
        return groups[groupId].members;
    }

    /**
     * @notice Generate conversation ID for direct messages
     * @dev Deterministic: same ID regardless of who initiates
     */
    function getDirectConversationId(
        address user1,
        address user2
    ) external pure returns (bytes32) {
        if (user1 < user2) {
            return keccak256(abi.encodePacked("dm", user1, user2));
        }
        return keccak256(abi.encodePacked("dm", user2, user1));
    }
}
