#include <stdio.h>
#include <sys/socket.h>
#include <unistd.h>
#include <arpa/inet.h>
#include <string.h>
#include <stdlib.h>
#include <sys/time.h>
#include <errno.h>

#define PORT 5050

struct header {
    uint16_t ID;
    uint16_t flags; // QR(1), Opcode(4), AA(1), TC(1), RD(1), RA(1), Z(3), RCODE(4)
    uint16_t QDCOUNT;
    uint16_t ANCOUNT;
    uint16_t NSCOUNT;
    uint16_t ARCOUNT;
};

struct resolve {
    char *domain_name;
    uint16_t lenght;
};

struct sockaddr_in client_addr;

void set_timeout(int socket, int seconds) {
    struct timeval tv;
    tv.tv_sec = seconds;
    tv.tv_usec = 0;

    if (setsockopt(socket, SOL_SOCKET, SO_RCVTIMEO, &tv, sizeof(tv)) < 0) {
        perror("setsockopt");
    }
}

int manage_request(unsigned char *buffer, int length, char **domain_name) {
    int i = 0;
    int pos = 0;

    char *result = (char *)malloc(length + 2);
    if (result == NULL || result == 0) {
        perror("malloc");
        return 1;
    }

    while (i < length && buffer[i] != 0) {
        int label_len = buffer[i];
        i++;

        if (i > 1) {
            result[pos++] = '.';
        }

        if (i + label_len > length) {
            break;
        }

        for (int j = 0; j < label_len; j++) {
            result[pos++] = buffer[i + j];
        }

        i += label_len;
    }

    result[pos++] = '.';
    result[pos++] = '\0';
    *domain_name = result;

    return i + 2;
}

int intialize() {
    int sock;

    sock = socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP);
    if (sock <= 0) {
        perror("socket");
        return 1;
    }

    struct sockaddr_in server_addr;
    server_addr.sin_family = AF_INET;
    server_addr.sin_port = htons(PORT);
    server_addr.sin_addr.s_addr = INADDR_ANY;
    if (bind(sock, (struct sockaddr *)&server_addr, sizeof(server_addr)) < 0) {
        perror("bind");
        return 1;
    }

    set_timeout(sock, 5600);

    return sock;
}

struct resolve receive(int sock, unsigned char *buffer, int buffer_size) {
    socklen_t client_len = sizeof(client_addr);

    int bytes = recvfrom(sock, buffer, buffer_size, 0, (struct sockaddr *)&client_addr, &client_len);
    if (bytes <= 0) {
        if (errno == EAGAIN || errno == EWOULDBLOCK) {
            return (struct resolve){.domain_name = NULL, .lenght = 0xFFFF};    
        }
        
        return (struct resolve){.domain_name = NULL, .lenght = 0};
    }

    char *domain_name;
    uint16_t qname_len = 4 + manage_request(buffer + sizeof(struct header), bytes - sizeof(struct header), &domain_name);
    if (qname_len <= 0) {
        perror("manage_request");
        return (struct resolve){.domain_name = NULL, .lenght = 0};
    }

    return (struct resolve){.domain_name = domain_name, .lenght = qname_len};
}

int respond(int sock, unsigned char *buffer, int qlen, uint16_t responses, uint8_t error) {
    struct header *hdr = (struct header *)buffer;
    socklen_t client_len = sizeof(struct sockaddr_in);

    hdr->ANCOUNT = htons(responses);
    hdr->ARCOUNT = 0;
    hdr->flags = htons((ntohs(hdr->flags)  & 0xFFF0) | 0x8080 | (error & 0x000F));

    int responseBytes = sendto(sock, buffer, sizeof(struct header) + qlen, 0, (struct sockaddr *)&client_addr, client_len);
    if (responseBytes <= 0) {
        perror("sendto");
        return 1;
    }

    return 0;
}

int stop(int sock) {
    close(sock);
    return 0;
}

int get_port() {
    return PORT;
}

