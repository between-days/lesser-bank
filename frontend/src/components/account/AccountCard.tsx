import { Card, Text, Group, Stack, MantineShadow, Center, Button } from '@mantine/core';
import { useHover } from '@mantine/hooks';
import { useRouter } from 'next/router';
import { getFormattedAccountNumber, getFormattedBsb, getIconForAccountType } from '@/UIUtils';
import { AccountType } from '@/interfaces';
import { AmountText } from '../shared/AmountText';

export interface AccountCardProps {
    accountName: string | undefined
    accountType: AccountType
    balanceCents: number
    availableBalanceCents: number
    accountNumber: string
    bsb: string,
    onHover: boolean
}

export default function AccountCard({ accountName, accountType, accountNumber, availableBalanceCents, bsb, onHover }: AccountCardProps) {
    const router = useRouter()
    const { hovered, ref } = useHover();

    let shadow: MantineShadow | undefined = "sm"
    let withBorder: boolean = false
    if (onHover) {
        shadow = hovered ? "xl" : "sm"
        withBorder = hovered ? false : true
    }

    if (!accountName || accountName.length == 0) {
        accountName = accountType.toUpperCase() + " ACCOUNT"
    }

    const onClick = () => {
        router.push(`/accounts/${accountNumber}`)
    }

    return (
        <Card ref={ref} shadow={shadow} radius="lg" withBorder={withBorder} maw="20rem">
            <Card.Section >
                <Center
                    sx={(theme) => ({
                        height: '0.5rem',
                        backgroundImage: theme.fn.gradient({ from: 'red', to: 'orange', deg: 45 }),
                        color: theme.white,
                    })}
                >
                </Center>
            </Card.Section>
            <Stack spacing="xl">
                <Group position='apart' mt='lg'>
                    <Group position='left'>
                        {getIconForAccountType(accountType)}
                        <Stack spacing={0}>
                            <Text weight={500} fz="lg">{accountName}</Text>
                            <Group>
                                <Text color="dimmed" fz="sm">
                                    {getFormattedAccountNumber(accountNumber)}
                                </Text>
                                <Text color="dimmed" fz="sm">
                                    {getFormattedBsb(bsb)}
                                </Text>
                            </Group>
                        </Stack>
                    </Group>
                </Group>
                <Group position='apart'>
                    <AmountText amountCents={availableBalanceCents} />
                    <Button variant="light" onClick={onClick}>
                        Details
                    </Button>
                </Group>
            </Stack>
        </Card>
    );
}