#!/usr/bin/env python3
"""
Lair Chat Python Client Example

A simple Python client demonstrating how to interact with the Lair Chat REST API.
This example shows user authentication, room management, and message handling.

Requirements:
    pip install requests

Usage:
    python lair_chat_client.py
"""

import requests
import json
import time
import threading
from datetime import datetime
from typing import Optional, List, Dict, Any


class LairChatClient:
    """
    Python client for Lair Chat REST API

    This client provides a simple interface to interact with Lair Chat's
    REST API, including authentication, room management, and messaging.
    """

    def __init__(self, base_url: str = "http://127.0.0.1:8082/api/v1"):
        """
        Initialize the Lair Chat client

        Args:
            base_url: Base URL for the Lair Chat API
        """
        self.base_url = base_url.rstrip('/')
        self.session = requests.Session()
        self.session.headers.update({
            'Content-Type': 'application/json',
            'User-Agent': 'LairChatPythonClient/1.0'
        })

        self.token: Optional[str] = None
        self.user_info: Optional[Dict[str, Any]] = None
        self.current_room: Optional[Dict[str, Any]] = None
        self.message_polling = False
        self.polling_thread: Optional[threading.Thread] = None

    def _make_request(self, method: str, endpoint: str, data: Optional[Dict] = None) -> Dict[str, Any]:
        """
        Make an authenticated request to the API

        Args:
            method: HTTP method (GET, POST, PUT, DELETE)
            endpoint: API endpoint (without base URL)
            data: Request payload for POST/PUT requests

        Returns:
            Response data as dictionary

        Raises:
            requests.exceptions.RequestException: For network errors
            ValueError: For API errors
        """
        url = f"{self.base_url}{endpoint}"

        kwargs = {'timeout': 30}
        if data is not None:
            kwargs['json'] = data

        try:
            response = self.session.request(method, url, **kwargs)
            response.raise_for_status()

            result = response.json()

            # Check for API-level errors
            if not result.get('success', True):
                error_msg = result.get('error', {}).get('message', 'Unknown API error')
                raise ValueError(f"API Error: {error_msg}")

            return result

        except requests.exceptions.RequestException as e:
            raise requests.exceptions.RequestException(f"Network error: {e}")
        except json.JSONDecodeError:
            raise ValueError("Invalid JSON response from server")

    def test_connection(self) -> bool:
        """
        Test connection to the Lair Chat API

        Returns:
            True if connection successful, False otherwise
        """
        try:
            response = self._make_request('GET', '/health')
            return response.get('data', {}).get('status') == 'ok'
        except Exception as e:
            print(f"Connection test failed: {e}")
            return False

    def register(self, username: str, email: str, password: str) -> bool:
        """
        Register a new user account

        Args:
            username: Desired username
            email: User's email address
            password: User's password

        Returns:
            True if registration successful, False otherwise
        """
        try:
            data = {
                'username': username,
                'email': email,
                'password': password
            }

            response = self._make_request('POST', '/auth/register', data)
            print(f"Registration successful for user: {username}")
            return True

        except Exception as e:
            print(f"Registration failed: {e}")
            return False

    def login(self, identifier: str, password: str) -> bool:
        """
        Login with username/email and password

        Args:
            identifier: Username or email
            password: User's password

        Returns:
            True if login successful, False otherwise
        """
        try:
            data = {
                'identifier': identifier,
                'password': password
            }

            response = self._make_request('POST', '/auth/login', data)

            self.token = response.get('access_token')
            self.user_info = response.get('user')

            if self.token:
                # Set authorization header for future requests
                self.session.headers['Authorization'] = f'Bearer {self.token}'
                print(f"Login successful! Welcome, {self.user_info.get('username', 'User')}")
                return True
            else:
                print("Login failed: No token received")
                return False

        except Exception as e:
            print(f"Login failed: {e}")
            return False

    def logout(self):
        """Logout and clear authentication"""
        self.stop_message_polling()

        if 'Authorization' in self.session.headers:
            del self.session.headers['Authorization']

        self.token = None
        self.user_info = None
        self.current_room = None

        print("Logged out successfully")

    def get_profile(self) -> Optional[Dict[str, Any]]:
        """
        Get current user's profile

        Returns:
            User profile data or None if error
        """
        try:
            response = self._make_request('GET', '/users/profile')
            return response.get('data')
        except Exception as e:
            print(f"Failed to get profile: {e}")
            return None

    def get_rooms(self) -> List[Dict[str, Any]]:
        """
        Get list of available rooms

        Returns:
            List of room dictionaries
        """
        try:
            response = self._make_request('GET', '/rooms')
            return response.get('data', [])
        except Exception as e:
            print(f"Failed to get rooms: {e}")
            return []

    def create_room(self, name: str, description: str = "") -> Optional[Dict[str, Any]]:
        """
        Create a new chat room

        Args:
            name: Room name
            description: Room description

        Returns:
            Created room data or None if error
        """
        try:
            data = {
                'name': name,
                'description': description
            }

            response = self._make_request('POST', '/rooms', data)
            room_data = response.get('data')

            print(f"Room '{name}' created successfully")
            return room_data

        except Exception as e:
            print(f"Failed to create room: {e}")
            return None

    def join_room(self, room_id: str) -> bool:
        """
        Join a chat room

        Args:
            room_id: ID of the room to join

        Returns:
            True if successful, False otherwise
        """
        try:
            response = self._make_request('POST', f'/rooms/{room_id}/join')
            print(f"Joined room successfully")
            return True
        except Exception as e:
            print(f"Failed to join room: {e}")
            return False

    def get_messages(self, room_id: str, limit: int = 50) -> List[Dict[str, Any]]:
        """
        Get messages from a room

        Args:
            room_id: Room ID to get messages from
            limit: Maximum number of messages to retrieve

        Returns:
            List of message dictionaries
        """
        try:
            params = f"?room_id={room_id}&limit={limit}"
            response = self._make_request('GET', f'/messages{params}')
            return response.get('data', [])
        except Exception as e:
            print(f"Failed to get messages: {e}")
            return []

    def send_message(self, room_id: str, content: str) -> bool:
        """
        Send a message to a room

        Args:
            room_id: Target room ID
            content: Message content

        Returns:
            True if successful, False otherwise
        """
        try:
            data = {
                'room_id': room_id,
                'content': content
            }

            response = self._make_request('POST', '/messages', data)
            return True

        except Exception as e:
            print(f"Failed to send message: {e}")
            return False

    def start_message_polling(self, room_id: str, interval: float = 2.0):
        """
        Start polling for new messages in a room

        Args:
            room_id: Room ID to monitor
            interval: Polling interval in seconds
        """
        if self.message_polling:
            self.stop_message_polling()

        self.message_polling = True
        self.current_room_id = room_id

        def poll_messages():
            last_message_count = 0

            while self.message_polling:
                try:
                    messages = self.get_messages(room_id, limit=10)

                    if len(messages) > last_message_count:
                        # New messages available
                        new_messages = messages[last_message_count:]
                        for msg in new_messages:
                            self._display_message(msg)
                        last_message_count = len(messages)

                    time.sleep(interval)

                except Exception as e:
                    print(f"Error polling messages: {e}")
                    time.sleep(interval)

        self.polling_thread = threading.Thread(target=poll_messages, daemon=True)
        self.polling_thread.start()

        print(f"Started message polling for room {room_id}")

    def stop_message_polling(self):
        """Stop message polling"""
        if self.message_polling:
            self.message_polling = False
            if self.polling_thread:
                self.polling_thread.join(timeout=1.0)
            print("Stopped message polling")

    def _display_message(self, message: Dict[str, Any]):
        """
        Display a message in a formatted way

        Args:
            message: Message dictionary
        """
        timestamp = message.get('timestamp', '')
        author = message.get('author', 'Unknown')
        content = message.get('content', '')

        # Format timestamp
        try:
            dt = datetime.fromisoformat(timestamp.replace('Z', '+00:00'))
            time_str = dt.strftime('%H:%M:%S')
        except:
            time_str = timestamp

        print(f"[{time_str}] {author}: {content}")


def interactive_demo():
    """
    Interactive demo of the Lair Chat Python client
    """
    print("=" * 60)
    print("🐍 Lair Chat Python Client Demo")
    print("=" * 60)

    # Initialize client
    client = LairChatClient()

    # Test connection
    print("\n🔍 Testing API connection...")
    if not client.test_connection():
        print("❌ Failed to connect to Lair Chat API")
        print("Please ensure the server is running on http://127.0.0.1:8082")
        return

    print("✅ API connection successful!")

    # Authentication
    print("\n🔐 Authentication")
    choice = input("Do you want to (l)ogin or (r)egister? [l/r]: ").lower().strip()

    if choice == 'r':
        print("\n📝 Registration")
        username = input("Username: ")
        email = input("Email: ")
        password = input("Password: ")

        if client.register(username, email, password):
            print("✅ Registration successful! You can now login.")
        else:
            print("❌ Registration failed")
            return

    print("\n🔑 Login")
    username = input("Username or email: ")
    password = input("Password: ")

    if not client.login(username, password):
        print("❌ Login failed")
        return

    try:
        # Main menu loop
        while True:
            print("\n" + "=" * 40)
            print("📋 Main Menu")
            print("1. List rooms")
            print("2. Create room")
            print("3. Join room and chat")
            print("4. View profile")
            print("5. Logout")
            print("=" * 40)

            choice = input("Select option [1-5]: ").strip()

            if choice == '1':
                print("\n📂 Available Rooms:")
                rooms = client.get_rooms()
                if rooms:
                    for i, room in enumerate(rooms, 1):
                        print(f"{i}. {room['name']} - {room.get('description', 'No description')}")
                else:
                    print("No rooms available")

            elif choice == '2':
                print("\n🏗️ Create New Room")
                name = input("Room name: ")
                description = input("Room description (optional): ")

                room = client.create_room(name, description)
                if room:
                    print(f"✅ Room '{name}' created with ID: {room['id']}")

            elif choice == '3':
                print("\n💬 Join Room and Chat")
                rooms = client.get_rooms()
                if not rooms:
                    print("No rooms available")
                    continue

                print("Available rooms:")
                for i, room in enumerate(rooms, 1):
                    print(f"{i}. {room['name']}")

                try:
                    room_idx = int(input("Select room number: ")) - 1
                    if 0 <= room_idx < len(rooms):
                        selected_room = rooms[room_idx]

                        # Join room
                        if client.join_room(selected_room['id']):
                            print(f"\n💬 Joined room: {selected_room['name']}")
                            print("Recent messages:")

                            # Show recent messages
                            messages = client.get_messages(selected_room['id'], limit=10)
                            for msg in messages[-5:]:  # Show last 5 messages
                                client._display_message(msg)

                            # Start message polling
                            client.start_message_polling(selected_room['id'])

                            print("\n📝 Type messages (press Enter to send, 'quit' to leave room):")

                            while True:
                                try:
                                    message = input("> ")
                                    if message.lower() == 'quit':
                                        break
                                    elif message.strip():
                                        client.send_message(selected_room['id'], message)
                                except KeyboardInterrupt:
                                    break

                            client.stop_message_polling()
                            print(f"Left room: {selected_room['name']}")
                    else:
                        print("Invalid room selection")
                except ValueError:
                    print("Invalid input")

            elif choice == '4':
                print("\n👤 User Profile")
                profile = client.get_profile()
                if profile:
                    print(f"Username: {profile.get('username')}")
                    print(f"Email: {profile.get('email')}")
                    print(f"Role: {profile.get('role', 'User')}")
                    print(f"Created: {profile.get('created_at', 'Unknown')}")

            elif choice == '5':
                client.logout()
                break

            else:
                print("Invalid option")

    except KeyboardInterrupt:
        print("\n\n👋 Goodbye!")
        client.logout()


if __name__ == "__main__":
    try:
        interactive_demo()
    except KeyboardInterrupt:
        print("\n\n👋 Goodbye!")
    except Exception as e:
        print(f"\n❌ Unexpected error: {e}")
